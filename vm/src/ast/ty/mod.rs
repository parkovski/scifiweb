use std::fmt::{self, Debug, Display};
use std::mem;
use std::iter::Iterator;
use serde::ser::{Serialize, Serializer, SerializeTupleVariant};
use erased_serde::Serialize as ErasedSerialize;
use util::graph_cell::*;
use compile::{TokenValue, TokenSpan};
use super::var::Variable;
use super::errors::*;
use super::*;

mod array;
mod collectable;
mod event;
mod function;
mod object;
mod user;

pub use self::array::*;
pub use self::collectable::*;
pub use self::event::*;
pub use self::function::*;
pub use self::object::*;
pub use self::user::*;

/// Primitive types usable as-is.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub enum PrimitiveType {
  Void,
  Option,
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
      Option => "option",
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

  pub fn iter() -> impl Iterator<Item = PrimitiveType> {
    struct PrimitiveTypeIter {
      next: Option<PrimitiveType>,
    }

    impl Iterator for PrimitiveTypeIter {
      type Item = PrimitiveType;
      fn next(&mut self) -> Option<PrimitiveType> {
        use self::PrimitiveType::*;
        let next = self.next;
        self.next = match next {
          Some(Void) => Some(Option),
          Some(Option) => Some(Text),
          Some(Text) => Some(LocalizedText),
          Some(LocalizedText) => Some(Integer),
          Some(Integer) => Some(Decimal),
          Some(Decimal) => Some(DateTime),
          Some(DateTime) => Some(TimeSpan),
          Some(TimeSpan) => Some(Object),
          Some(Object) => Some(Array),
          Some(Array) => None,
          None => None,
        };
        next
      }
    }

    PrimitiveTypeIter { next: Some(PrimitiveType::Void) }
  }
}

impl Display for PrimitiveType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Debug)]
pub struct PrimitiveTypeSet<'a> {
  void: GraphRef<'a, Type<'a>>,
  option: GraphRef<'a, Type<'a>>,
  text: GraphRef<'a, Type<'a>>,
  localized_text: GraphRef<'a, Type<'a>>,
  integer: GraphRef<'a, Type<'a>>,
  decimal: GraphRef<'a, Type<'a>>,
  date_time: GraphRef<'a, Type<'a>>,
  time_span: GraphRef<'a, Type<'a>>,
  object: GraphRef<'a, Type<'a>>,
  array: GraphRef<'a, Type<'a>>,
}

impl<'a> PrimitiveTypeSet<'a> {
  pub fn new(map: &FxHashMap<Arc<str>, GraphCell<Type<'a>>>) -> Self {
    PrimitiveTypeSet {
      void: map.get(PrimitiveType::Void.as_str()).unwrap().asleep(),
      option: map.get(PrimitiveType::Option.as_str()).unwrap().asleep(),
      text: map.get(PrimitiveType::Text.as_str()).unwrap().asleep(),
      localized_text: map.get(PrimitiveType::LocalizedText.as_str()).unwrap().asleep(),
      integer: map.get(PrimitiveType::Integer.as_str()).unwrap().asleep(),
      decimal: map.get(PrimitiveType::Decimal.as_str()).unwrap().asleep(),
      date_time: map.get(PrimitiveType::DateTime.as_str()).unwrap().asleep(),
      time_span: map.get(PrimitiveType::TimeSpan.as_str()).unwrap().asleep(),
      object: map.get(PrimitiveType::Object.as_str()).unwrap().asleep(),
      array: map.get(PrimitiveType::Array.as_str()).unwrap().asleep(),
    }
  }

  pub fn void(&self) -> GraphRef<'a, Type<'a>> {
    self.void
  }

  pub fn option(&self) -> GraphRef<'a, Type<'a>> {
    self.option
  }

  pub fn text(&self) -> GraphRef<'a, Type<'a>> {
    self.text
  }

  pub fn localized_text(&self) -> GraphRef<'a, Type<'a>> {
    self.localized_text
  }

  pub fn integer(&self) -> GraphRef<'a, Type<'a>> {
    self.integer
  }

  pub fn decimal(&self) -> GraphRef<'a, Type<'a>> {
    self.decimal
  }

  pub fn date_time(&self) -> GraphRef<'a, Type<'a>> {
    self.date_time
  }

  pub fn time_span(&self) -> GraphRef<'a, Type<'a>> {
    self.time_span
  }

  pub fn object(&self) -> GraphRef<'a, Type<'a>> {
    self.object
  }

  pub fn array(&self) -> GraphRef<'a, Type<'a>> {
    self.array
  }
}

/// "Generic" types that form the base
/// of user defined instances.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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

  pub fn insert_empty_type<'a>(
    &self,
    ast: GraphRefMut<'a, Ast<'a>>,
    name: TokenValue<Arc<str>>
  ) -> Result<()>
  {
    match *self {
      BaseCustomType::Array => {
        return Err(ErrorKind::InvalidOperation(
          "empty custom array type is not valid - use PrimitiveType::Array"
        ).into());
      }
      BaseCustomType::Object => { Ast::insert_type(ast, Object::new(name))?; }
      BaseCustomType::Collectable => { Ast::insert_type(ast, Collectable::new(name))?; }
      BaseCustomType::CollectableGroup => { Ast::insert_type(ast, CollectableGroup::new(name))?; }
      BaseCustomType::User => { Ast::insert_type(ast, User::new(name))?; }
      BaseCustomType::UserGroup => { Ast::insert_type(ast, UserGroup::new(name))?; }
      BaseCustomType::Event => { Ast::insert_type(ast, Event::new(name))?; }
      BaseCustomType::RemoteEvent => { Ast::insert_type(ast, RemoteEvent::new(name))?; }
      BaseCustomType::Function => { Ast::insert_type(ast, Function::new(name))?; }
      BaseCustomType::RemoteFunction => { Ast::insert_type(ast, RemoteFunction::new(name))?; }
    }
    Ok(())
  }
}

impl Display for BaseCustomType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

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
  + CustomTypeAsSerialize
  + Named
  + SourceItem
  + 'a
{
  fn init_cyclic(
    &mut self,
    _self_ref: GraphRef<'a, Self>,
    _ast_ref: GraphRef<'a, Ast<'a>>
  ) where Self: Sized {}

  fn base_type(&self) -> BaseCustomType;
  fn capabilities(&self) -> TypeCapability;

  fn is_sub_type_of(&self, _ty: &CustomType<'a>) -> bool { false }
  fn property(&self, _name: &str) -> Option<GraphRef<'a, Variable<'a>>> { None }
}

pub trait CustomTypeAsSerialize {
  fn as_serialize(&self) -> &ErasedSerialize;
}

impl<T> CustomTypeAsSerialize for T where T: ErasedSerialize {
  fn as_serialize(&self) -> &ErasedSerialize {
    self
  }
}

impl<'a> Serialize for CustomType<'a> {
  fn serialize<S: Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
    self.as_serialize().serialize(serializer)
  }
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
    let base_type_ptr = ty as *const CustomType<'a> as *const usize;
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

impl<'a, T: CustomType<'a> + 'a> From<T> for Type<'a> {
  fn from(custom: T) -> Self {
    Type::Custom(Box::new(custom))
  }
}

pub trait SubType<'a, T: CustomType<'a>>: CustomType<'a> {
  fn super_type(&self) -> Option<GraphRef<'a, T>>;
  fn assign_super_type_internal(&mut self, super_type: GraphRef<'a, T>);
  fn set_super_type(&mut self, super_type: GraphRef<'a, T>) -> Result<()> {
    if let Some(ref st) = self.super_type() {
      if st.awake().name() == super_type.awake().name() {
        return Ok(());
      } else {
        return Err(ErrorKind::ConflictingSuperType(
          self.name().value().clone(),
          st.awake().name().value().clone(),
          super_type.awake().name().clone(),
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
  fn name(&self) -> &TokenValue<Arc<str>> {
    match *self {
      Type::Primitive(_, ref name) => name,
      Type::Custom(ref ty) => ty.name(),
    }
  }

  fn item_name(&self) -> &'static str {
    match *self {
      Type::Primitive(ty, _) => ty.as_str(),
      Type::Custom(ref ty) => ty.item_name(),
    }
  }
}

impl_name_traits!(@all Type, <'a>);
named_display!(Type, <'a>);

impl<'a> SourceItem for Type<'a> {
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

impl<'a> Serialize for Type<'a> {
  fn serialize<S: Serializer>(&self, serializer: S)
    -> ::std::result::Result<S::Ok, S::Error>
  {
    match *self {
      Type::Primitive(ref t, _) => {
        let mut tv = serializer.serialize_tuple_variant("Type", 0, "Primitive", 1)?;
        tv.serialize_field(t)?;
        tv.end()
      }
      Type::Custom(ref t) => {
        let mut tv = serializer.serialize_tuple_variant("Type", 1, "Custom", 1)?;
        tv.serialize_field(t.as_serialize())?;
        tv.end()
      }
    }
  }
}
