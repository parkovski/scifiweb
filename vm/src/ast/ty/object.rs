use std::sync::Arc;
use util::graph_cell::*;
use compile::{TokenSpan, TokenValue};
use super::*;

#[derive(Debug, Serialize)]
pub struct Object<'ast> {
  name: TokenValue<Arc<str>>,
  dynamic: bool,
  scope: GraphCell<Scope<'ast>>,
  super_type: Option<GraphRef<'ast, Object<'ast>>>,
}

impl<'ast> Object<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    Ast::insert_cast_type(
      ast,
      Object {
        name,
        dynamic: false,
        scope: Scope::child(parent_scope, ScopeKind::TYPE | ScopeKind::RECURSIVE, span),
        super_type: None,
      }
    )
  }
}

type_macros!(
  Object<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('ast,)
);

impl<'ast> SourceItem for Object<'ast> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    self.scope.awake_mut().resolve()
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'ast> CastType<'ast> for Object<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Object;
}

impl<'ast> CustomType<'ast> for Object<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Object
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::PROPERTIES | TypeCapability::OWNED | TypeCapability::INHERIT
  }

  fn property(&self, _name: &str) -> Option<GraphRef<'ast, Variable<'ast>>> {
    None
  }

  fn is_sub_type_of(&self, _ty: &CustomType<'ast>) -> bool {
    false
  }
}

impl<'ast> SubType<'ast, Object<'ast>> for Object<'ast> {
  fn super_type(&self) -> Option<GraphRef<'ast, Object<'ast>>> {
    self.super_type
  }

  fn assign_super_type_internal(&mut self, super_type: GraphRef<'ast, Object<'ast>>) {
    self.super_type = Some(super_type);
  }
}
