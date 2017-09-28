use std::sync::Arc;
use util::graph_cell::*;
use compile::{TokenSpan, TokenValue};
use ast::var::*;
use super::*;

#[derive(Debug, Serialize)]
pub struct Object<'a> {
  name: TokenValue<Arc<str>>,
  dynamic: bool,
  scope: GraphCell<Scope<'a>>,
  super_type: Option<GraphRef<'a, Object<'a>>>,
}

impl<'a> Object<'a> {
  pub fn new(name: TokenValue<Arc<str>>, parent_scope: GraphRef<'a, Scope<'a>>)
    -> Self
  {
    let span = name.span().clone();
    Object {
      name,
      dynamic: false,
      scope: Scope::child(parent_scope, span),
      super_type: None,
    }
  }
}

type_macros!(
  Object<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for Object<'a> {
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

impl<'a> CastType<'a> for Object<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Object;
}

impl<'a> CustomType<'a> for Object<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Object
  }

  fn capabilities(&self) -> TypeCapability {
    TC_PROPERTIES | TC_OWNED | TC_INHERIT
  }

  fn property(&self, _name: &str) -> Option<GraphRef<'a, Variable<'a>>> {
    None
  }

  fn is_sub_type_of(&self, _ty: &CustomType<'a>) -> bool {
    false
  }
}

impl<'a> SubType<'a, Object<'a>> for Object<'a> {
  fn super_type(&self) -> Option<GraphRef<'a, Object<'a>>> {
    self.super_type
  }

  fn assign_super_type_internal(&mut self, super_type: GraphRef<'a, Object<'a>>) {
    self.super_type = Some(super_type);
  }
}
