use std::sync::Arc;
use util::graph_cell::*;
use compile::{TokenSpan, TokenValue};
use super::*;

/// An ordered sequence of values, optionally with custom bounds
/// and a specific type.
#[derive(Debug)]
pub struct Array<'a> {
  name: TokenValue<Arc<str>>,
  ty: Option<GraphRef<'a, Type<'a>>>,
  /// This can't be usize because we don't know what hardware this will need
  /// to be shared with. Also, given what this system is, if you want
  /// a bigger array, you're wrong.
  max_length: Option<u32>,
}

impl_name_traits!((<'a>) Array (<'a>));
named_display!((<'a>) Array (<'a>));

impl<'a> Array<'a> {
  pub fn new(name: TokenValue<Arc<str>>) -> Self {
    Array { name, ty: None, max_length: None }
  }

  pub fn make_name(length: Option<u32>, type_name: Option<&str>) -> Arc<str> {
    let mut s = String::from("array");
    if let Some(length) = length {
      s.push_str(" x ");
      s.push_str(&length.to_string());
    }
    if let Some(type_name) = type_name {
      s.push_str(" of ");
      s.push_str(type_name);
    }
    s.into()
  }
}

impl<'a> SourceItem for Array<'a> {
  fn source_name(&self) -> &TokenValue<Arc<str>> {
    &self.name
  }

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
    TC_OWNED
  }
}
