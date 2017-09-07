use std::sync::Arc;
use compile::{TokenSpan, TokenValue};
use util::graph_cell::*;
use super::*;
use super::errors::*;
use super::ty::*;

#[derive(Debug)]
pub struct Variable<'a> {
  name: TokenValue<Arc<str>>,
  ty: ItemRef<'a, Type<'a>>,
}

impl<'a> Variable<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    ty: ItemRef<'a, Type<'a>>,
  ) -> Self
  {
    Variable { name, ty }
  }

  pub fn ty(&self) -> Option<GraphRef<Type<'a>>> {
    self.ty.item()
  }
}

impl<'a> Named for Variable<'a> {
  fn name(&self) -> &str {
    &self.name
  }

  fn item_name(&self) -> &'static str {
    "property"
  }
}

named_display!((<'a>)Variable(<'a>));

impl<'a> SourceItem for Variable<'a> {
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
