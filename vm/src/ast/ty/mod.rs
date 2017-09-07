use std::fmt::{Debug, Display};
use std::path::Path;
use std::mem;
use util::graph_cell::*;
use compile::{TokenValue, TokenSpan};
use super::var::Variable;
use super::errors::*;
use super::*;

mod array;
//mod callable;
mod collectable;
mod object;
//mod remote;
mod user;

pub use self::array::*;
pub use self::collectable::*;
pub use self::user::*;

/// Primitive types usable as-is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveType {
  Void,
  Switch,
  Text,
  LocalizedText,
  Integer,
  Decimal,
  DateTime,
  TimeSpan,
  Object,
  Array,
}

impl PrimitiveType {
  pub fn as_str(&self) -> &'static str {
    use self::PrimitiveType::*;
    match *self {
      Void => "void",
      Switch => "switch",
      Text => "text",
      LocalizedText => "localized text",
      Integer => "integer",
      Decimal => "decimal",
      DateTime => "datetime",
      TimeSpan => "timespan",
      Object => "object",
      Array => "array",
    }
  }

  pub fn insert_all<'a>(ast: &mut Ast<'a>) {
    use self::PrimitiveType::*;
    let types = [
      Void, Switch,
      Text, LocalizedText,
      Integer, Decimal,
      DateTime, TimeSpan,
      Object, Array
    ];
    let span = TokenSpan::new(Arc::new(Path::new("internal").into()));
    for ty in &types {
      let name = ast.shared_string(ty.as_str());
      ast.insert(Type::Primitive(*ty, TokenValue::new(name, span.clone()))).unwrap();
    }
  }
}

impl Named for PrimitiveType {
  fn name(&self) -> &str {
    self.as_str()
  }

  fn item_name(&self) -> &'static str {
    ""
  }
}

named_display!(PrimitiveType);

/// "Generic" types that form the base
/// of user defined instances.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BaseCustomType {
  Array,
  Object,
  Collectable,
  CollectableGroup,
  User,
  UserGroup,
  Event,
  RemoteEvent,
  Function,
  RemoteFunction,
}

impl BaseCustomType {
  pub fn as_str(&self) -> &'static str {
    use self::BaseCustomType::*;
    match *self {
      Array => "array",
      Object => "object",
      Collectable => "collectable",
      CollectableGroup => "collectable group",
      User => "user",
      UserGroup => "user group",
      Event => "event",
      RemoteEvent => "remote event",
      Function => "function",
      RemoteFunction => "remote function",
    }
  }

  pub fn into_empty_type<'a>(self, name: TokenValue<Arc<str>>) -> Type<'a> {
    use self::BaseCustomType::*;
    match self {
      Array => unimplemented!(),
      Object => unimplemented!(),
      Collectable => unimplemented!(),
      CollectableGroup => unimplemented!(),
      User => unimplemented!(),
      UserGroup => unimplemented!(),
      Event => unimplemented!(),
      RemoteEvent => unimplemented!(),
      Function => unimplemented!(),
      RemoteFunction => unimplemented!(),
    }
  }
}

impl Named for BaseCustomType {
  fn name(&self) -> &str {
    self.as_str()
  }

  fn item_name(&self) -> &'static str {
    ""
  }
}

named_display!(BaseCustomType);

bitflags! {
  #[derive(Default)]
  pub struct TypeCapability: u32 {
    /// The type has custom properties.
    const TC_PROPERTIES                        = 0b00000001;
    /// The type can run custom code.
    const TC_EXECUTE                           = 0b00000010;
    /// Instances of the type belong to
    /// another entity. When that entity
    /// is deleted, these should be too.
    const TC_OWNED                             = 0b00000100;
    /// This type receives notifications
    /// that cause the program to resume.
    const TC_NOTIFY_RECEIVER                   = 0b00001000;
    /// This type needs to set up a
    /// web endpoint to receive its
    /// notifications.
    const TC_NOTIFY_ENDPOINT                   = 0b00010000;
    /// This type may inherit
    /// from another type.
    const TC_INHERIT                           = 0b00100000;
  }
}

pub trait CustomType<'a>
  : Debug
  + Display
  + Named
  + SourceItem
{
  fn base_type(&self) -> BaseCustomType;
  fn capabilities(&self) -> TypeCapability;

  fn super_type(&self) -> Option<GraphRef<'a, CustomType<'a>>> { None }
  fn property(&self, _name: &str) -> Option<GraphRef<'a, Variable<'a>>> { None }

  /// For casting
  fn _self_ptr(&self) -> *const usize { self as *const _ as *const usize }
}

/// This is safe as long as no type provides
/// a mismatched `BASE_TYPE`. The size assertion
/// should help catch that kind of error.
pub trait CastType<'a>: CustomType<'a> + Sized {
  const BASE_TYPE: BaseCustomType;

  /// Returns `None` if the cast is incorrect.
  fn try_cast<'b>(ty: &'b CustomType<'a>) -> Option<&'b Self> {
    if ty.base_type() != Self::BASE_TYPE {
      return None;
    }
    debug_assert!(mem::size_of::<Self>() == mem::size_of_val(ty));
    let base_type_ptr = ty._self_ptr();
    unsafe { mem::transmute(base_type_ptr) }
  }

  // Rust doesn't know this but going from &mut -> & -> &mut
  // for the same thing is ok.
  #[allow(mutable_transmutes)]
  fn try_cast_mut<'b>(ty: &'b mut CustomType<'a>) -> Option<&'b mut Self> {
    Self::try_cast(ty).map(|r| unsafe { mem::transmute(r) })
  }

  /// Panics if the cast is incorrect.
  fn cast<'b>(ty: &'b CustomType<'a>) -> &'b Self {
    Self::try_cast(ty).expect("Mismatched type cast")
  }

  #[allow(mutable_transmutes)]
  fn cast_mut<'b>(ty: &'b mut CustomType<'a>) -> &'b mut Self {
    unsafe { mem::transmute(Self::cast(ty)) }
  }
}

impl<'a, T: CustomType<'a> + SourceItem> Named for T {
  fn name(&self) -> &str {
    &self.source_name().value()
  }

  fn item_name(&self) -> &'static str {
    self.base_type().as_str()
  }
}

impl<'a, T: CustomType<'a> + 'a> From<T> for Type<'a> {
  fn from(custom: T) -> Self {
    Type::Custom(Box::new(custom))
  }
}

pub trait SubType<'a, T: SourceItem>: SourceItem {
  fn super_type(&self) -> Option<GraphRef<'a, T>>;
  fn assign_super_type_internal(&mut self, super_type: GraphRef<'a, T>);
  fn set_super_type(&mut self, super_type: GraphRef<'a, T>) -> Result<()> {
    if let Some(ref st) = self.super_type() {
      if st.awake().name() == super_type.awake().name() {
        return Ok(());
      } else {
        return Err(ErrorKind::ConflictingSuperType(
          self.source_name().value().clone(),
          st.awake().source_name().value().clone(),
          super_type.awake().source_name().value().clone(),
        ).into());
      }
    }
    self.assign_super_type_internal(super_type);
    Ok(())
  }
}

#[derive(Debug)]
pub enum Type<'a> {
  Primitive(PrimitiveType, TokenValue<Arc<str>>),
  Custom(Box<CustomType<'a> + 'a>),
}

impl<'a> Type<'a> {
  pub fn is_primitive(&self) -> bool {
    self.as_primitive().is_some()
  }

  pub fn as_primitive(&self) -> Option<PrimitiveType> {
    match *self {
      Type::Primitive(t, _) => Some(t),
      Type::Custom(_) => None,
    }
  }

  pub fn is_custom(&self) -> bool {
    self.as_custom().is_some()
  }

  pub fn as_custom(&self) -> Option<&CustomType<'a>> {
    match *self {
      Type::Primitive(_, _) => None,
      Type::Custom(ref t) => Some(t.as_ref()),
    }
  }

  pub fn as_custom_mut(&mut self) -> Option<&mut CustomType<'a>> {
    match *self {
      Type::Primitive(_, _) => None,
      Type::Custom(ref mut t) => Some(t.as_mut()),
    }
  }
}

impl<'a> Named for Type<'a> {
  fn name(&self) -> &str {
    match *self {
      Type::Primitive(ref ty, _) => ty.name(),
      Type::Custom(ref ty) => ty.name(),
    }
  }

  fn item_name(&self) -> &'static str {
    match *self {
      Type::Primitive(ty, _) => ty.item_name(),
      Type::Custom(ref ty) => ty.item_name(),
    }
  }
}

impl_name_traits!((<'a>) Type (<'a>), all);
named_display!((<'a>) Type (<'a>));

impl<'a> SourceItem for Type<'a> {
  fn source_name(&self) -> &TokenValue<Arc<str>> {
    match *self {
      Type::Primitive(_, ref name) => name,
      Type::Custom(ref ty) => ty.source_name(),
    }
  }

  fn span(&self) -> &TokenSpan {
    match *self {
      Type::Primitive(_, ref name) => name.span(),
      Type::Custom(ref ty) => ty.span(),
    }
  }

  fn resolve(&mut self) -> Result<()> {
    match *self {
      Type::Primitive(_, _) => Ok(()),
      Type::Custom(ref mut ty) => ty.resolve(),
    }
  }

  fn typecheck(&mut self) -> Result<()> {
    match *self {
      Type::Primitive(_, _) => Ok(()),
      Type::Custom(ref mut ty) => ty.typecheck(),
    }
  }
}
