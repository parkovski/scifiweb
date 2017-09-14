use std::sync::Arc;
use std::default::Default;
use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use fxhash::FxHashMap;
use util::{SharedStrings, InsertGraphCell};
use util::graph_cell::*;
use compile::{TokenSpan, TokenValue};

#[macro_use]
pub mod macros;
pub mod ty;
pub mod var;

use self::ty::*;
use self::var::*;
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

/// For ItemRef resolution.
pub trait Owner<'a, T: SourceItem + 'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, T>>;
  fn find(&self, name: &str) -> Option<GraphRef<'a, T>> {
    self.find_mut(name).map(|gr| gr.asleep_ref())
  }
}

// =====

#[derive(Debug, Serialize)]
pub struct ItemRef<'a, T: SourceItem + 'a> {
  name: TokenValue<Arc<str>>,
  item: Option<GraphRef<'a, T>>,
}

#[derive(Debug, Serialize)]
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

      pub fn with_item(name: TokenValue<Arc<str>>, item: $graph_ref<'a, T>) -> Self {
        $t {
          name,
          item: Some(item),
        }
      }

      pub fn resolve<O>(&mut self, owner: &O) -> Result<$graph_ref<'a, T>>
      where O: Owner<'a, T> + Debug + 'a
      {
        if let Some(ref item) = self.item {
          return Ok(item.clone());
        }

        match owner.$find(&self.name) {
          Some(r) => {
            self.item = Some(r.clone());
            Ok(r)
          }
          None => Err(ErrorKind::NotDefined(
            self.name.clone(),
            "item"
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
        ""
      }
    }

    named_display!((<'a, T: SourceItem + 'a>)$t(<'a, T>));
  );
}

item_ref_impls!(ItemRef, GraphRef, awake, find);
item_ref_impls!(ItemRefMut, GraphRefMut, awake, find_mut);

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

#[derive(Debug, Serialize)]
pub struct Ast<'a> {
  types: FxHashMap<Arc<str>, GraphCell<Type<'a>>>,
  #[serde(skip)]
  array_names: FxHashMap<ArrayName, Arc<str>>,
  global_scope: GraphCell<Scope<'a>>,
  strings: SharedStrings,
  /// The path "(internal)" for things with no code location.
  #[serde(skip)]
  internal_path: Arc<PathBuf>,
}

impl<'a> Ast<'a> {
  pub fn new() -> Box<GraphCell<Self>> {
    let ast = box GraphCell::new(Ast {
      types: Default::default(),
      array_names: Default::default(),
      global_scope: Scope::new(),
      strings: SharedStrings::new(),
      internal_path: Arc::new(Path::new("(internal)").into()),
    });
    let mut ast_ref = ast.awake_mut();
    let pt_span = TokenSpan::new(ast_ref.internal_path());
    for pt in PrimitiveType::iter() {
      let name = ast_ref.shared_string(pt.name());
      let tkval = TokenValue::new(name.clone(), pt_span.clone());
      let ty = Type::Primitive(pt, tkval);
      ast_ref.types.insert(name, GraphCell::new(ty));
    }
    ast
  }

  pub fn shared_string(&self, s: &str) -> Arc<str> {
    self.strings.get(s)
  }

  pub fn internal_path(&self) -> Arc<PathBuf> {
    self.internal_path.clone()
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
    trace!("Resolve references");
    self.resolution_step(SourceItem::resolve)?;
    trace!("Typecheck");
    self.resolution_step(SourceItem::typecheck)
  }

  pub fn insert_type<T>(this: GraphRefMut<'a, Ast<'a>>, ty: T) -> Result<GraphRefMut<'a, Type<'a>>>
  where T: CustomType<'a> + CastType<'a> + 'a
  {
    let name = ty.source_name().value().clone();
    let self_ref = this.asleep_ref();
    let gr = this
      .awake_mut()
      .types
      .insert_graph_cell(name, Type::Custom(Box::new(ty)));
    let type_ref = match gr {
      Ok(type_ref) => type_ref,
      Err(ty) => return Err(
        ErrorKind::DuplicateDefinition(
          ty.source_name().clone(),
          ty.item_name(),
        ).into()
      ),
    };
    let t_mut = type_ref.map(|r| T::cast_mut(r.as_custom_mut().unwrap()));
    let t_ref = t_mut.asleep_ref();
    t_mut.awake_mut().init_cyclic(t_ref, self_ref);
    Ok(type_ref)
  }

  pub fn get_array(
    this: GraphRefMut<'a, Ast<'a>>,
    name: ArrayName,
  ) -> GraphRef<'a, Type<'a>>
  {
    let opt_str_name = this.awake().array_names.get(&name).map(|n| n.clone());
    let str_name = if let Some(stored) = opt_str_name {
      stored
    } else {
      let stored: Arc<str> = name.to_string().into();
      this.awake_mut().array_names.insert(name.clone(), stored.clone());
      stored
    };
    let opt_array = <Self as Owner<Type>>::find(&this.awake(), &str_name);
    if let Some(array) = opt_array {
      // It's either the primitive type "array" or something with a name
      // only an array can have.
      debug_assert!(
        array
          .awake()
          .as_custom()
          .map(|a| a.base_type() == BaseCustomType::Array)
          .unwrap_or(true)
      );
      array
    } else {
      let tv = TokenValue::new(str_name, TokenSpan::new(this.awake().internal_path.clone()));
      let ty = name.type_name.map(|n| ItemRef::new(n.clone()));
      let array = Array::new(tv, ty, name.length);
      Self::insert_type(this, array).unwrap().asleep_ref()
    }
  }
}

impl<'a> Owner<'a, Type<'a>> for Ast<'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Type<'a>>> {
    self.types.get(name).map(|t| t.asleep_mut())
  }
}

impl<'a, T> Owner<'a, T> for Ast<'a>
where
  T: CustomType<'a> + CastType<'a> + 'a
{
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
*/

mod errors {
  // ?????
  #![allow(unused_doc_comment)]
  use std::sync::Arc;
  use compile::{TokenSpan, TokenValue};

  error_chain! {
    errors {
      InvalidOperation(operation: &'static str) {
        description("invalid operation")
        display("internal error: invalid operation {}", operation)
      }

      NotDefined(name: TokenValue<Arc<str>>, typ: &'static str) {
        description("item not defined")
        display("{}: no definition for {} '{}'", name.span(), typ, name.value())
      }

      DuplicateDefinition(name: TokenValue<Arc<str>>, typ: &'static str) {
        description("item already defined")
        display("{}: {} '{}' already defined", name.span(), typ, name.value())
      }

      TypeResolution(expected: Arc<str>, found: TokenValue<Arc<str>>) {
        description("type resolution mismatch")
        display(
          "{}: expected type '{}', found '{}' instead",
          found.span(),
          &expected,
          found.value()
        )
      }

      ConflictingSuperType(
        ty: Arc<str>,
        parent: Arc<str>,
        conflicting_parent: TokenValue<Arc<str>>
      )
      {
        description("conflicting super type")
        display(
          "{}: can't set super type of '{}' to '{}' because it already has super type '{}'",
          conflicting_parent.span(),
          &ty,
          conflicting_parent.value(),
          &parent
        )
      }

      ValueOutOfRange(value: String, reason: &'static str, location: TokenSpan) {
        description("value out of range")
        display("{}: value '{}' out of range: {}", &location, &value, reason)
      }
    }
  }
}

pub use self::errors::{
  Error as AstError,
  ErrorKind as AstErrorKind,
  Result as AstResult,
  ResultExt as AstResultExt,
};
