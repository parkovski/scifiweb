use std::sync::Arc;
use fxhash::FxHashMap;
use util::graph_cell::*;
use util::InsertGraphCell;
use compile::{TokenSpan, TokenValue};
use ast::var::Variable;
use super::*;

#[derive(Debug)]
pub struct Object<'a> {
  name: TokenValue<Arc<str>>,
  dynamic: bool,
  properties: FxHashMap<Arc<str>, GraphCell<Variable<'a>>>,
  super_type: Option<ItemRef<'a, Object<'a>>>,
}

impl_name_traits!((<'a>) Object (<'a>));
named_display!((<'a>) Object (<'a>));

impl<'a> Object<'a> {
  pub fn new(name: TokenValue<Arc<str>>) -> Self {
    Object {
      name,
      dynamic: false,
      properties: Default::default(),
      super_type: None,
    }
  }
}

impl<'a> SourceItem for Object<'a> {
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

  fn property(&self, name: &str) -> Option<GraphRef<'a, Variable<'a>>> {
    self.properties.get(name).map(|p| p.asleep())
  }

  fn super_type(&self) -> Option<GraphRef<'a, CustomType<'a>>> {
    None
  }
}

impl<'a> SubType<'a, Object<'a>> for Object<'a> {
  fn super_type(&self) -> Option<GraphRef<'a, Object<'a>>> {
    None
  }

  fn assign_super_type_internal(&mut self, super_type: GraphRef<'a, Object<'a>>) {
    //
  }
}

impl<'a> Owner<'a, Variable<'a>> for Object<'a> {
  fn insert(&mut self, p: Variable<'a>) -> Result<GraphRefMut<'a, Variable<'a>>> {
    self.properties
      .insert_graph_cell(p.source_name().value().clone(), p)
      .map_err(|p| ErrorKind::DuplicateDefinition(
        p.source_name().value().clone(),
        "property"
      ).into())
  }

  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Variable<'a>>> {
    self.properties.get(name).map(|p| p.asleep_mut())
  }
}
