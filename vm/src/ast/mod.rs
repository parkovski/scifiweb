use std::sync::Arc;
use std::default::Default;
use std::fmt::{Debug, Display};
use fxhash::FxHashMap;
use util::{SharedStrings, InsertUnique};
use util::graph_cell::*;
use compile::{TokenSpan, TokenValue};

#[macro_use]
pub mod macros;
pub mod ty;
pub mod var;

use self::ty::*;
use self::errors::*;

// =====

pub trait Named: Debug + Display {
  fn name(&self) -> &str;
  fn item_name(&self) -> &'static str;
}

pub trait SourceItem: Named {
  fn source_name(&self) -> &TokenValue<Arc<str>>;
  fn span(&self) -> &TokenSpan;

  /// After this, the item knows all of its references
  /// exist, but doesn't yet know if they all fit together.
  fn resolve(&mut self) -> Result<()>;
  /// After this, the program is valid. The item knows
  /// where all its references are and that they are
  /// all correctly formed.
  fn typecheck(&mut self) -> Result<()>;
}

pub trait Owner<'a, T: SourceItem + 'a> {
  fn insert(&mut self, item: T) -> Result<GraphRefMut<'a, T>>;
  fn find(&self, name: &str) -> Option<GraphRefMut<'a, T>>;
}

pub trait RefOwner<'a, T, O>
where
  T: SourceItem + 'a,
  O: Owner<'a, T> + Debug + 'a,
{
  fn insert(&mut self, r: ItemRef<'a, T, O>) -> Result<()>;
  fn has_ref(&self, name: &str) -> bool;
}

// =====

#[derive(Debug, Clone)]
pub struct ItemRef<'a, T, O>
where
  T: SourceItem + 'a,
  O: Owner<'a, T> + Debug + 'a,
{
  name: TokenValue<Arc<str>>,
  owner: GraphRef<'a, O>,
  item: Option<GraphRef<'a, T>>,
}

#[derive(Debug, Clone)]
pub struct ItemRefMut<'a, T, O>
where
  T: SourceItem + 'a,
  O: Owner<'a, T> + Debug + 'a,
{
  name: TokenValue<Arc<str>>,
  owner: GraphRef<'a, O>,
  item: Option<GraphRefMut<'a, T>>,
}

impl<'a, T: SourceItem + 'a, O: Owner<'a, T> + Debug + 'a> ItemRef<'a, T, O> {
  pub fn new(name: TokenValue<Arc<str>>, owner: GraphRef<'a, O>) -> Self {
    ItemRef {
      name,
      owner,
      item: None,
    }
  }

  pub fn item(&self) -> Option<GraphRef<'a, T>> {
    self.item
  }
}

impl<'a, T: SourceItem + 'a, O: Owner<'a, T> + Debug + 'a> ItemRefMut<'a, T, O> {
  pub fn new(name: TokenValue<Arc<str>>, owner: GraphRef<'a, O>) -> Self {
    ItemRefMut {
      name,
      owner,
      item: None,
    }
  }

  pub fn item(&self) -> Option<GraphRef<'a, T>> {
    self.item.map(|r| r.asleep_ref())
  }

  pub fn item_mut(&mut self) -> Option<GraphRefMut<'a, T>> {
    self.item
  }
}

macro_rules! item_ref_impls {
  ($t:ident, ($($conv:tt)*)) => (
    impl<'a, T, O> Named for $t<'a, T, O>
    where
      T: SourceItem + 'a,
      O: Owner<'a, T> + Debug + 'a,
    {
      fn name(&self) -> &str {
        &self.name
      }

      fn item_name(&self) -> &'static str {
        "ref"
      }
    }

    named_display!((<'a, T: SourceItem + 'a, O: Owner<'a, T> + Debug + 'a>)$t(<'a, T, O>));

    impl<'a, T, O> SourceItem for $t<'a, T, O>
    where
      T: SourceItem + 'a,
      O: Owner<'a, T> + Debug + 'a,
    {
      fn source_name(&self) -> &TokenValue<Arc<str>> {
        &self.name
      }

      fn span(&self) -> &TokenSpan {
        self.name.span()
      }

      fn resolve(&mut self) -> Result<()> {
        match self.owner.awake().find(&self.name) {
          Some(item) => Ok(self.item = Some(item $($conv)*)),
          None => Err(ErrorKind::NotDefined(self.name.value().clone(), "ref").into()),
        }
      }

      fn typecheck(&mut self) -> Result<()> {
        Ok(())
      }
    }
  );
}

item_ref_impls!(ItemRef, (.asleep_ref()));
item_ref_impls!(ItemRefMut, ());

// =====

#[derive(Debug)]
pub struct Ast<'a> {
  types: FxHashMap<Arc<str>, GraphCell<Type<'a>>>,
  //globals: FxHashMap<Arc<str>, GraphCell<var::Var<'a>>>,
  strings: SharedStrings,
}

impl<'a> Ast<'a> {
  pub fn new() -> Box<GraphCell<Self>> {
    box GraphCell::new(Ast {
      types: Default::default(),
      //globals: Default::default(),
      strings: SharedStrings::new(),
    })
  }

  pub fn shared_string(&self, s: &str) -> Arc<str> {
    self.strings.get(s)
  }

  fn resolution_step<F>(&mut self, step: F) -> Result<()>
  where F: Fn(&mut (SourceItem + 'a)) -> Result<()>
  {
    for ty in self.types.values_mut() {
      (step)(&mut *ty.awake_mut())?;
    }
    Ok(())
  }

  pub fn typecheck(&mut self) -> Result<()> {
    self.resolution_step(SourceItem::resolve)?;
    self.resolution_step(SourceItem::typecheck)
  }
}

impl<'a> Owner<'a, Type<'a>> for Ast<'a> {
  fn insert(&mut self, ty: Type<'a>) -> Result<GraphRefMut<'a, Type<'a>>> {
    let name = ty.source_name().value().clone();
    let gc = GraphCell::new(ty);
    let r = gc.asleep_mut();
    self.types
      .insert_unique(name, gc)
      .map(|_| r)
      .map_err(
        |(name, ty)| ErrorKind::DuplicateDefinition(name, ty.awake().item_name()).into()
      )
  }

  fn find(&self, name: &str) -> Option<GraphRefMut<'a, Type<'a>>> {
    self.types.get(name).map(|t| t.asleep_mut())
  }
}

impl<'a, T> Owner<'a, T> for Ast<'a>
where
  T: CustomType<'a> + CastType<'a> + 'a
{
  fn insert(&mut self, ty: T) -> Result<GraphRefMut<'a, T>> {
    let name = ty.source_name().value().clone();
    let cell = GraphCell::new(Type::Custom(Box::new(ty)));
    let gr = cell.asleep_mut();
    match self.types.insert_unique(name, cell) {
      Ok(()) => Ok(gr.map(|r| T::cast_mut(r.as_custom_mut().unwrap()))),
      Err((name, _)) => Err(ErrorKind::DuplicateDefinition(name, "type").into())
    }
  }

  /// This uses the mutable reference, so no other
  /// references can be active when calling this.
  fn find(&self, name: &str) -> Option<GraphRefMut<'a, T>> {
    let gr = match self.types.get(name) {
      Some(ref cell) => cell.asleep_mut(),
      None => return None,
    };
    gr.map_opt(|t| t.as_custom_mut().and_then(T::try_cast_mut))
  }
}

/*
pub struct Upgrade {
  pub cost: GrpRef<Collectable, CollectableGroup>,
  pub cost_amount: Expression,
  pub self_amount_range: Range,
}

pub enum Redemption {
  ForCurrency,
  ForCollectable {
    self_amount: u32,
    cost: GrpRef<Collectable, CollectableGroup>,
    cost_amount: u32,
  }
}
/// A map contains correlations
/// between a constant value of any key
/// property on an entity and one value
/// property with variable expressions,
/// and must have an exhaustive branch
/// (or = expr).
pub struct Map {
  pub key_type: Type,
  pub value_type: Type,
  pub value_property: VarRef,
  pub branches: Vec<MapBranch>,
  pub default_branch: Expression,
}

pub struct MapBranch {
  pub key_property: VarRef,
  pub key_value: Constant,
  pub value_expr: Expression,
}

/// Do we create a random assortment
/// of collectables or a random assortment
/// of groups that contain the same
/// collectable?
pub enum RandomDistribution {
  Individual,
  /// The range indicates how many
  /// groups to make. The number of items
  /// per group will vary based on the
  /// weights so no matter the group
  /// distribution settings, it won't be
  /// possible to get a bunch of something
  /// that's set to be super rare.
  Group(Range),
}

pub enum RandomList {
  Weighted(Vec<(f32, Type)>),
  Unweighted(Vec<Type>),
}

/// A Random is a collectable generator.
/// It takes an amount expression
pub struct Random {
  pub distribution: RandomDistribution,
  pub item_type: Type,
  pub amount: Range,
  pub items: RandomList,
}
*/

mod errors {
  // ?????
  #![allow(unused_doc_comment)]
  use std::sync::Arc;

  error_chain! {
    errors {
      InvalidOperation(operation: &'static str) {
        description("invalid operation")
        display("invalid operation: {}", operation)
      }

      NotDefined(name: Arc<str>, typ: &'static str) {
        description("item not defined")
        display("no definition for {} '{}'", typ, &name)
      }

      DuplicateDefinition(name: Arc<str>, typ: &'static str) {
        description("item already defined")
        display("{} '{}' already defined", typ, &name)
      }

      TypeResolution(expected: Arc<str>, found: Arc<str>) {
        description("type resolution mismatch")
        display("expected type '{}', found '{}' instead", &expected, &found)
      }

      ConflictingSuperType(ty: Arc<str>, parent: Arc<str>, conflicting_parent: Arc<str>) {
        description("conflicting super type")
        display(
          "can't set super type of '{}' to '{}' because it already has super type '{}'",
          &ty,
          &conflicting_parent,
          &parent
        )
      }
    }
  }
}

pub use self::errors::{
  Error as AstError,
  ErrorKind as AstErrorKind,
  Result as AstResult,
  ResultExt as ResultExt,
};
