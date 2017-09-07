use std::sync::Arc;
use std::default::Default;
use std::fmt::{Debug, Display};
use fxhash::FxHashMap;
use util::{SharedStrings, InsertGraphCell};
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
  /// The name of the item, which must be unique
  /// among its type in its scope.
  fn name(&self) -> &str;

  /// The item type name, e.g. "collectable" or "variable".
  /// This can be an empty string, in which case
  /// only the name is shown.
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
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, T>>;
  fn find(&self, name: &str) -> Option<GraphRef<'a, T>> {
    self.find_mut(name).map(|gr| gr.asleep_ref())
  }
}

pub trait RefOwner<'a, T: SourceItem + 'a> {
  fn insert_ref(&mut self, r: ItemRef<'a, T>) -> Result<()>;
  fn has_ref(&self, name: &str) -> bool;
}

pub trait RefMutOwner<'a, T: SourceItem + 'a> {
  fn insert_ref_mut(&mut self, r: ItemRefMut<'a, T>) -> Result<()>;
  fn has_ref_mut(&self, name: &str) -> bool;
}

// =====

#[derive(Debug)]
pub struct ItemRef<'a, T: SourceItem + 'a> {
  name: TokenValue<Arc<str>>,
  item: Option<GraphRef<'a, T>>,
}

#[derive(Debug)]
pub struct ItemRefMut<'a, T: SourceItem + 'a> {
  name: TokenValue<Arc<str>>,
  item: Option<GraphRefMut<'a, T>>,
}

macro_rules! item_ref_impls {
  ($t:ident, $graph_ref:ident, $awake:ident, $find:ident) => (
    impl<'a, T: SourceItem + 'a> $t<'a, T> {
      pub fn new(name: TokenValue<Arc<str>>) -> Self {
        $t {
          name,
          item: None,
        }
      }

      pub fn with_item(item: $graph_ref<'a, T>) -> Self {
        $t {
          name: item.$awake().source_name().clone(),
          item: Some(item),
        }
      }

      pub fn resolve<O>(&mut self, owner: &O) -> Result<$graph_ref<'a, T>>
      where O: Owner<'a, T> + Debug + 'a
      {
        match owner.$find(&self.name) {
          Some(r) => {
            self.item = Some(r.clone());
            Ok(r)
          }
          None => Err(ErrorKind::NotDefined(
            self.name.value().clone(),
            "ref" // T::BASE_TYPE
          ).into()),
        }
      }

      pub fn source_name(&self) -> &TokenValue<Arc<str>> {
        &self.name
      }
    }

    impl<'a, T: SourceItem + 'a> Named for $t<'a, T> {
      fn name(&self) -> &str {
        &self.name
      }

      fn item_name(&self) -> &'static str {
        "ref"
      }
    }

    named_display!((<'a, T: SourceItem + 'a>)$t(<'a, T>));
  );
}

item_ref_impls!(ItemRef, GraphRef, awake, find);
item_ref_impls!(ItemRefMut, GraphRefMut, awake_ref, find_mut);

impl<'a, T: SourceItem + 'a> ItemRef<'a, T> {
  pub fn item(&self) -> Option<GraphRef<'a, T>> {
    self.item
  }
}

impl<'a, T: SourceItem + 'a> ItemRefMut<'a, T> {
  pub fn item(&self) -> Option<GraphRef<'a, T>> {
    self.item.map(|r| r.asleep_ref())
  }

  pub fn item_mut(&mut self) -> Option<GraphRefMut<'a, T>> {
    self.item
  }

  pub fn into_item_ref(self) -> ItemRef<'a, T> {
    ItemRef {
      name: self.name,
      item: self.item.map(|i| i.asleep_ref()),
    }
  }
}

// =====

#[derive(Debug)]
pub struct Ast<'a> {
  types: FxHashMap<Arc<str>, GraphCell<Type<'a>>>,
  //globals: FxHashMap<Arc<str>, GraphCell<var::Var<'a>>>,
  strings: SharedStrings,
}

impl<'a> Ast<'a> {
  pub fn new() -> Box<GraphCell<Self>> {
    let ast = box GraphCell::new(Ast {
      types: Default::default(),
      //globals: Default::default(),
      strings: SharedStrings::new(),
    });
    PrimitiveType::insert_all(&mut ast.awake_mut());
    ast
  }

  pub fn shared_string(&self, s: &str) -> Arc<str> {
    self.strings.get(s)
  }

  fn resolution_step<F>(&self, step: F) -> Result<()>
  where F: Fn(&mut (SourceItem + 'a)) -> Result<()>
  {
    for ty in self.types.values() {
      (step)(&mut *ty.awake_mut())?;
    }
    Ok(())
  }

  pub fn typecheck(&self) -> Result<()> {
    self.resolution_step(SourceItem::resolve)?;
    self.resolution_step(SourceItem::typecheck)
  }
}

impl<'a> Owner<'a, Type<'a>> for Ast<'a> {
  fn insert(&mut self, ty: Type<'a>) -> Result<GraphRefMut<'a, Type<'a>>> {
    let name = ty.source_name().value().clone();
    self.types
      .insert_graph_cell(name, ty)
      .map_err(|ty| ErrorKind::DuplicateDefinition(
        ty.source_name().value().clone(),
        ty.item_name()
      ).into())
  }

  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Type<'a>>> {
    self.types.get(name).map(|t| t.asleep_mut())
  }
}

impl<'a, T> Owner<'a, T> for Ast<'a>
where
  T: CustomType<'a> + CastType<'a> + 'a
{
  fn insert(&mut self, ty: T) -> Result<GraphRefMut<'a, T>> {
    let name = ty.source_name().value().clone();
    let ty = Type::Custom(Box::new(ty));
    self.types
      .insert_graph_cell(name, ty)
      .map(|gr| gr.map(|r| T::cast_mut(r.as_custom_mut().unwrap())))
      .map_err(|ty| ErrorKind::DuplicateDefinition(
        ty.source_name().value().clone(),
        ty.item_name(),
      ).into())
  }

  /// This uses the mutable reference, so no other
  /// references can be active when calling this.
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, T>> {
    let gr = match self.types.get(name) {
      Some(ref cell) => cell.asleep_mut(),
      None => return None,
    };
    gr.map_opt(|t| t.as_custom_mut().and_then(T::try_cast_mut))
  }

  fn find(&self, name: &str) -> Option<GraphRef<'a, T>> {
    let gr = match self.types.get(name) {
      Some(ref cell) => cell.asleep(),
      None => return None,
    };
    gr.map_opt(|t| t.as_custom().and_then(T::try_cast))
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
