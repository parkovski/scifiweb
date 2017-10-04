use std::sync::Arc;
use util::graph_cell::*;
use compile::{TokenSpan, TokenValue};
use super::*;

#[derive(Debug, Serialize)]
pub struct Function<'ast> {
  name: TokenValue<Arc<str>>,
  param_scope: GraphCell<Scope<'ast>>,
  local_scope: Later<GraphCell<Scope<'ast>>>,
}

impl<'ast> Function<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    let f = Ast::insert_cast_type(
      ast,
      Function {
        name,
        param_scope: Scope::child(parent_scope, ScopeKind::FN_PARAM, span.clone()),
        local_scope: Later::new(),
      }
    )?;
    let mut fmut = f.awake_mut();
    let param_scope = fmut.param_scope.asleep();
    Later::set(
      &mut fmut.local_scope,
      Scope::child(param_scope, ScopeKind::FN_LOCAL, span)
    );
    Ok(f)
  }
}

type_macros!(
  Function<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped(for local_scope: 'ast in)
);

impl<'ast> SourceItem for Function<'ast> {
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

impl<'ast> CastType<'ast> for Function<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Function;
}

impl<'ast> CustomType<'ast> for Function<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Function
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::EXECUTE
  }
}

#[derive(Debug, Serialize)]
pub struct RemoteFunction<'ast> {
  name: TokenValue<Arc<str>>,
  param_scope: GraphCell<Scope<'ast>>,
}

impl<'ast> RemoteFunction<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    Ast::insert_cast_type(
      ast,
      RemoteFunction {
        name,
        param_scope: Scope::child(parent_scope, ScopeKind::FN_PARAM, span),
      }
    )
  }
}

type_macros!(
  RemoteFunction<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped(for param_scope: 'ast in)
);

impl<'ast> SourceItem for RemoteFunction<'ast> {
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

impl<'ast> CastType<'ast> for RemoteFunction<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::RemoteFunction;
}

impl<'ast> CustomType<'ast> for RemoteFunction<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::RemoteFunction
  }

  fn capabilities(&self) -> TypeCapability {
    Default::default()
  }
}
