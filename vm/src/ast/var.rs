use std::sync::Arc;
use compile::{TokenSpan, TokenValue};
use util::graph_cell::*;
use super::*;
use super::errors::*;
use super::ty::*;

pub struct Var(u32);

#[derive(Debug)]
pub struct Property<'a> {
  name: TokenValue<Arc<str>>,
  ty: ItemRef<'a, Type<'a>, Ast<'a>>,
}

impl<'a> Property<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    ty: ItemRef<'a, Type<'a>, Ast<'a>>,
  ) -> Self
  {
    Property { name, ty }
  }

  pub fn ty(&self) -> Option<GraphRef<Type<'a>>> {
    self.ty.item()
  }
}

impl<'a> Named for Property<'a> {
  fn name(&self) -> &str {
    &self.name
  }

  fn item_name(&self) -> &'static str {
    "property"
  }
}

named_display!((<'a>)Property(<'a>));

impl<'a> SourceItem for Property<'a> {
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
