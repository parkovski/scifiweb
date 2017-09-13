use std::sync::Arc;
use util::graph_cell::*;
use ast::var::Variable;
use compile::{TokenSpan, TokenValue};
use super::*;

#[derive(Debug, Serialize)]
pub struct Function<'a> {
  name: TokenValue<Arc<str>>,
  params: Vec<GraphCell<Variable<'a>>>,
  //scope: GraphCell<Scope>,
}

impl_name_traits!((<'a>) Function (<'a>));
named_display!((<'a>) Function (<'a>));

impl<'a> Function<'a> {
  pub fn new(name: TokenValue<Arc<str>>) -> Self {
    Function {
      name,
      params: Vec::new(),
    }
  }
}

impl<'a> SourceItem for Function<'a> {
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

impl<'a> CastType<'a> for Function<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Function;
}

impl<'a> CustomType<'a> for Function<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Function
  }

  fn capabilities(&self) -> TypeCapability {
    TC_EXECUTE
  }
}

#[derive(Debug, Serialize)]
pub struct RemoteFunction<'a> {
  name: TokenValue<Arc<str>>,
  params: Vec<GraphCell<Variable<'a>>>,
}

impl_name_traits!((<'a>) RemoteFunction (<'a>));
named_display!((<'a>) RemoteFunction (<'a>));

impl<'a> RemoteFunction<'a> {
  pub fn new(name: TokenValue<Arc<str>>) -> Self {
    RemoteFunction {
      name,
      params: Vec::new(),
    }
  }
}

impl<'a> SourceItem for RemoteFunction<'a> {
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

impl<'a> CastType<'a> for RemoteFunction<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::RemoteFunction;
}

impl<'a> CustomType<'a> for RemoteFunction<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::RemoteFunction
  }

  fn capabilities(&self) -> TypeCapability {
    Default::default()
  }
}
