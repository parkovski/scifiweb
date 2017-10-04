use std::sync::Arc;
use std::fmt::{self, Display};
use compile::{TokenSpan, TokenValue};
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct ArrayName {
  pub length: Option<u32>,
  pub type_name: Option<TokenValue<Arc<str>>>,
}

impl ArrayName {
  pub fn new(length: Option<u32>, type_name: Option<TokenValue<Arc<str>>>) -> Self {
    ArrayName { length, type_name }
  }
}

impl Display for ArrayName {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match (self.length, &self.type_name) {
      (Some(ref len), &Some(ref name)) => write!(f, "array x {} of {}", len, &name.value()),
      (Some(len), &None) => write!(f, "array x {}", len),
      (None, &Some(ref name)) => write!(f, "array of {}", &name.value()),
      (None, &None) => write!(f, "array"),
    }
  }
}

/// An ordered sequence of values, optionally with custom bounds
/// and a specific type.
#[derive(Debug, Serialize)]
pub struct Array<'a> {
  name: TokenValue<Arc<str>>,
  ty: Option<ItemRef<'a, Type<'a>>>,
  /// TODO: asynchronously loaded big arrays?
  max_length: Option<u32>,
  scope: GraphCell<Scope<'a>>,
}

impl<'a> Array<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    ty: Option<ItemRef<'a, Type<'a>>>,
    max_length: Option<u32>,
    parent_scope: GraphRef<'a, Scope<'a>>,
  ) -> Self
  {
    let span = name.span().clone();
    Array {
      name,
      ty,
      max_length,
      scope: Scope::child(parent_scope, ScopeKind::TYPE, span)
    }
  }
}

type_macros!(
  Array<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for Array<'a> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> CastType<'a> for Array<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Array;
}

impl<'a> CustomType<'a> for Array<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Array
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::OWNED
  }
}
