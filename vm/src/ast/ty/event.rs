use std::sync::Arc;
use util::graph_cell::*;
use ast::var::Variable;
use compile::{TokenSpan, TokenValue};
use super::*;

#[derive(Debug, Serialize)]
pub struct Event<'a> {
  name: TokenValue<Arc<str>>,
  params: Vec<GraphCell<Variable<'a>>>,
  scope: GraphCell<Scope<'a>>,
}

impl<'a> Event<'a> {
  pub fn new(name: TokenValue<Arc<str>>, parent_scope: GraphRef<'a, Scope<'a>>) -> Self {
    let span = name.span().clone();
    Event {
      name,
      params: Vec::new(),
      scope: Scope::child(parent_scope, span),
    }
  }
}

type_macros!(
  Event<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for Event<'a> {
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

impl<'a> CastType<'a> for Event<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Event;
}

impl<'a> CustomType<'a> for Event<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Event
  }

  fn capabilities(&self) -> TypeCapability {
    TC_EXECUTE | TC_NOTIFY_RECEIVER | TC_NOTIFY_ENDPOINT
  }
}

#[derive(Debug, Serialize)]
pub struct RemoteEvent<'a> {
  name: TokenValue<Arc<str>>,
  params: Vec<GraphCell<Variable<'a>>>,
  scope: GraphCell<Scope<'a>>,
}

impl<'a> RemoteEvent<'a> {
  pub fn new(name: TokenValue<Arc<str>>, parent_scope: GraphRef<'a, Scope<'a>>) -> Self {
    let span = name.span().clone();
    RemoteEvent {
      name,
      params: Vec::new(),
      scope: Scope::child(parent_scope, span),
    }
  }
}

type_macros!(
  RemoteEvent<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for RemoteEvent<'a> {
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

impl<'a> CastType<'a> for RemoteEvent<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::RemoteEvent;
}

impl<'a> CustomType<'a> for RemoteEvent<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::RemoteEvent
  }

  fn capabilities(&self) -> TypeCapability {
    Default::default()
  }
}
