use std::sync::Arc;
use util::graph_cell::*;
use ast::var::Variable;
use compile::{TokenSpan, TokenValue};
use super::*;

#[derive(Debug, Serialize)]
pub struct Event<'ast> {
  name: TokenValue<Arc<str>>,
  params: Vec<GraphCell<Variable<'ast>>>,
  scope: GraphCell<Scope<'ast>>,
}

impl<'ast> Event<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    Ast::insert_cast_type(
      ast,
      Event {
        name,
        params: Vec::new(),
        scope: Scope::child(parent_scope, ScopeKind::TYPE, span),
      }
    )
  }
}

type_macros!(
  Event<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('ast,)
);

impl<'ast> SourceItem for Event<'ast> {
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

impl<'ast> CastType<'ast> for Event<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Event;
}

impl<'ast> CustomType<'ast> for Event<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Event
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::EXECUTE | TypeCapability::NOTIFY_RECEIVER | TypeCapability::NOTIFY_ENDPOINT
  }
}

#[derive(Debug, Serialize)]
pub struct RemoteEvent<'ast> {
  name: TokenValue<Arc<str>>,
  params: Vec<GraphCell<Variable<'ast>>>,
  scope: GraphCell<Scope<'ast>>,
}

impl<'ast> RemoteEvent<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    Ast::insert_cast_type(
      ast,
      RemoteEvent {
        name,
        params: Vec::new(),
        scope: Scope::child(parent_scope, ScopeKind::TYPE, span),
      }
    )
  }
}

type_macros!(
  RemoteEvent<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('ast,)
);

impl<'ast> SourceItem for RemoteEvent<'ast> {
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

impl<'ast> CastType<'ast> for RemoteEvent<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::RemoteEvent;
}

impl<'ast> CustomType<'ast> for RemoteEvent<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::RemoteEvent
  }

  fn capabilities(&self) -> TypeCapability {
    Default::default()
  }
}
