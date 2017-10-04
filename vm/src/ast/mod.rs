use std::sync::Arc;
use std::default::Default;
use std::fmt::{self, Debug, Display};
use std::path::{Path, PathBuf};
use serde::ser::{Serialize, Serializer};
use fxhash::FxHashMap;
use util::{SharedStrings, InsertGraphCell};
use util::graph_cell::*;
use util::later::Later;
use compile::{TokenSpan, TokenValue};

pub mod ty;
pub mod var;
pub mod expr;

use self::ty::*;
use self::var::*;
use self::errors::*;

// =====

pub trait Named: Debug + Display {
  /// The name of the item, which must be unique
  /// among its type in its scope.
  fn name(&self) -> &TokenValue<Arc<str>>;

  /// The item type name, e.g. "collectable" or "variable".
  /// This can be an empty string, in which case
  /// only the name is shown.
  fn item_name(&self) -> &'static str { "" }
}

pub trait SourceItem: Debug + Display {
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
pub trait Owner<'a, T: Named + ?Sized + 'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, T>>;
  fn find(&self, name: &str) -> Option<GraphRef<'a, T>> {
    self.find_mut(name).map(|gr| gr.asleep_ref())
  }
}

impl<'a, T: Named + ?Sized + 'a> Debug for Owner<'a, T> + 'a {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("Owner")
  }
}

// =====

macro_rules! item_ref_impls {
  ($t:ident, $res_later:ident, $graph_ref:ident, $find:ident) => (

    #[derive(Debug)]
    pub enum $res_later<'a, T: Debug + ?Sized + 'a, O: Debug + 'a> {
      Unresolved(O),
      Resolved($graph_ref<'a, T>),
    }

    impl<'a, T: Debug + ?Sized + 'a, O: Debug + 'a> $res_later<'a, T, O> {
      pub fn resolve<F>(&mut self, find: F, name: &TokenValue<Arc<str>>)
        -> Result<()>
      where F: FnOnce(&O, &str) -> Option<$graph_ref<'a, T>>
      {
        let item = match *self {
          $res_later::Resolved(_) => return Ok(()),
          $res_later::Unresolved(ref owner) => {
            let item = find(owner, name.value());
            match item {
              Some(r) => r,
              None => return Err(ErrorKind::NotDefined(
                name.clone(),
                "item"
              ).into()),
            }
          }
        };

        *self = $res_later::Resolved(item.clone());
        Ok(())
      }

      pub fn unwrap(&self) -> $graph_ref<'a, T> {
        match *self {
          $res_later::Resolved(item) => item,
          _ => panic!("Unwrap called on unresolved ResolveLater"),
        }
      }
    }

    impl<'a, T: Debug + ?Sized + 'a, O: Debug + 'a> Serialize for $res_later<'a, T, O> {
      fn serialize<S: Serializer>(&self, serializer: S)
        -> ::std::result::Result<S::Ok, S::Error>
      {
        match *self {
          $res_later::Unresolved(ref o)
            => serializer.serialize_str(&format!("Unresolved({:?})", o)),
          $res_later::Resolved(ref t)
            => serializer.serialize_str(&format!("Resolved({:?})", t)),
        }
      }
    }

    #[derive(Debug, Serialize)]
    pub struct $t<'a, T: Named + ?Sized + 'a> {
      name: TokenValue<Arc<str>>,
      item: $res_later<'a, T, GraphRef<'a, Owner<'a, T>>>,
    }

    impl<'a, T: Named + ?Sized + 'a> $t<'a, T> {
      pub fn new(
        name: TokenValue<Arc<str>>,
        owner: GraphRef<'a, Owner<'a, T>>
      ) -> Self
      {
        $t {
          name,
          item: $res_later::Unresolved(owner),
        }
      }

      pub fn with_item(name: TokenValue<Arc<str>>, item: $graph_ref<'a, T>) -> Self {
        $t {
          name,
          item: $res_later::Resolved(item),
        }
      }

      pub fn resolve(&mut self) -> Result<()> {
        let name = self.name.clone();
        self.item.resolve(|owner, name| owner.awake().$find(name), &name)
      }

      pub fn owner(&self) -> Option<GraphRef<'a, Owner<'a, T>>> {
        match self.item {
          $res_later::Unresolved(ref owner) => Some(owner.clone()),
          $res_later::Resolved(_) => None,
        }
      }
    }

    impl<'a, T: Named + ?Sized + 'a> Named for $t<'a, T> {
      fn name(&self) -> &TokenValue<Arc<str>> {
        &self.name
      }
    }

    named_display!($t<'a, T: Named + ?Sized + 'a>);
  );
}

item_ref_impls!(ItemRef, ResolveLater, GraphRef, find);
item_ref_impls!(ItemRefMut, ResolveLaterMut, GraphRefMut, find_mut);

impl<'a, T: Named + ?Sized + 'a> ItemRef<'a, T> {
  pub fn item(&self) -> Option<GraphRef<'a, T>> {
    match self.item {
      ResolveLater::Resolved(item) => Some(item),
      ResolveLater::Unresolved(_) => None,
    }
  }

  pub fn unwrap(&self) -> GraphRef<'a, T> {
    self.item().unwrap()
  }
}

impl<'a, T: Named + ?Sized + 'a> ItemRefMut<'a, T> {
  pub fn item(&self) -> Option<GraphRef<'a, T>> {
    match self.item {
      ResolveLaterMut::Resolved(item) => Some(item.asleep_ref()),
      ResolveLaterMut::Unresolved(_) => None,
    }
  }

  pub fn item_mut(&self) -> Option<GraphRefMut<'a, T>> {
    match self.item {
      ResolveLaterMut::Resolved(item) => Some(item),
      ResolveLaterMut::Unresolved(_) => None,
    }
  }

  pub fn unwrap(&self) -> GraphRefMut<'a, T> {
    self.item_mut().unwrap()
  }
}

// =====

#[derive(Debug, Serialize)]
pub struct Ast<'a> {
  types: FxHashMap<Arc<str>, GraphCell<Type<'a>>>,
  #[serde(skip)]
  primitive_types: Later<PrimitiveTypeSet<'a>>,
  #[serde(skip)]
  array_names: FxHashMap<ArrayName, Arc<str>>,
  scope: GraphCell<Scope<'a>>,
  strings: SharedStrings,
  /// The path "(internal)" for things with no code location.
  #[serde(skip)]
  internal_path: Arc<PathBuf>,
}

impl<'a> Ast<'a> {
  pub fn new() -> Box<GraphCell<Self>> {
    let ast = box GraphCell::new(Ast {
      types: Default::default(),
      primitive_types: Later::new(),
      array_names: Default::default(),
      scope: Scope::new(
        ScopeKind::GLOBAL,
        TokenSpan::new(Arc::new(Path::new("(global)").into())),
      ),
      strings: SharedStrings::new(),
      internal_path: Arc::new(Path::new("(internal)").into()),
    });
    {
      let mut ast_ref = ast.awake_mut();
      let pt_span = TokenSpan::new(ast_ref.internal_path());
      for pt in PrimitiveType::iter() {
        let name = ast_ref.shared_string(pt.as_str());
        let tkval = TokenValue::new(name.clone(), pt_span.clone());
        let scope = pt.make_scope(ast_ref.scope.asleep(), pt_span.clone());
        let ty = Type::Primitive(pt, tkval, scope);
        ast_ref.types.insert(name, GraphCell::new(ty));
      }
      let primitive_types = PrimitiveTypeSet::new(&ast_ref.types);
      Later::set(&mut ast_ref.primitive_types, primitive_types);
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
    (step)(&mut *self.scope.awake_mut())
  }

  pub fn typecheck(&self) -> Result<()> {
    trace!("Resolve references");
    self.resolution_step(SourceItem::resolve)?;
    trace!("Typecheck");
    self.resolution_step(SourceItem::typecheck)
  }

  pub fn primitive(&self) -> &PrimitiveTypeSet<'a> {
    &self.primitive_types
  }

  pub fn insert_type<T>(this: GraphRefMut<'a, Ast<'a>>, ty: T)
    -> Result<GraphRefMut<'a, Type<'a>>>
  where T: CustomType<'a> + CastType<'a> + 'a
  {
    let name = ty.name().value().clone();
    let self_ref = this.asleep_ref();
    let gr = this
      .awake_mut()
      .types
      .insert_graph_cell(name, Type::Custom(Box::new(ty)));
    let type_ref = match gr {
      Ok(type_ref) => type_ref,
      Err(ty) => return Err(
        ErrorKind::DuplicateDefinition(
          ty.name().clone(),
          ty.item_name(),
        ).into()
      ),
    };
    let t_mut = type_ref.map(|r| T::cast_mut(r.as_custom_mut().unwrap()));
    let t_ref = t_mut.asleep_ref();
    t_mut.awake_mut().init_cyclic(t_ref, self_ref);
    Ok(type_ref)
  }

  pub fn insert_cast_type<T>(this: GraphRefMut<'a, Ast<'a>>, ty: T)
    -> Result<GraphRefMut<'a, T>>
  where T: CustomType<'a> + CastType<'a> + 'a
  {
    Self::insert_type(this, ty)
      .map(
        |t| t.map(
          |r| T::cast_mut(r.as_custom_mut().unwrap())
        )
      )
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
      let ty = name.type_name.map(|n| ItemRef::new(n.clone(), this.asleep_ref()));
      let array = Array::new(tv, ty, name.length, this.awake().scope());
      Self::insert_type(this, array).unwrap().asleep_ref()
    }
  }
}

impl<'a> Owner<'a, Type<'a>> for Ast<'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Type<'a>>> {
    self.types.get(name).map(|t| t.asleep_mut())
  }
}

impl<'a> Owner<'a, CustomType<'a>> for Ast<'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, CustomType<'a>>> {
    self.types
      .get(name)
      .map(|gc| gc.asleep_mut().map_opt(Type::as_custom_mut))
      .unwrap_or(None)
  }

  fn find(&self, name: &str) -> Option<GraphRef<'a, CustomType<'a>>> {
    self.types
      .get(name)
      .map(|gc| gc.asleep().map_opt(Type::as_custom))
      .unwrap_or(None)
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

impl_scoped!('a, Ast<'a>);

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

      InvalidExpression(expr: String, span: TokenSpan) {
        description("invalid expression")
        display("{}: invalid expression '{}'", &span, &expr)
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
