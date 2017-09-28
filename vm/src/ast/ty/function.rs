use std::sync::Arc;
use util::graph_cell::*;
use ast::var::Variable;
use compile::{TokenSpan, TokenValue};
use super::*;

#[derive(Debug, Serialize)]
pub struct Function<'a> {
  name: TokenValue<Arc<str>>,
  params: Vec<GraphCell<Variable<'a>>>,
  scope: GraphCell<Scope<'a>>,
}

impl<'a> Function<'a> {
  pub fn new(name: TokenValue<Arc<str>>, parent_scope: GraphRef<'a, Scope<'a>>) -> Self {
    let span = name.span().clone();
    Function {
      name,
      params: Vec::new(),
      scope: Scope::child(parent_scope, span),
    }
  }
}

type_macros!(
  Function<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for Function<'a> {
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
  scope: GraphCell<Scope<'a>>,
}

impl<'a> RemoteFunction<'a> {
  pub fn new(name: TokenValue<Arc<str>>, parent_scope: GraphRef<'a, Scope<'a>>) -> Self {
    let span = name.span().clone();
    RemoteFunction {
      name,
      params: Vec::new(),
      scope: Scope::child(parent_scope, span),
    }
  }
}

type_macros!(
  RemoteFunction<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for RemoteFunction<'a> {
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
