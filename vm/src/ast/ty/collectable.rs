use std::sync::Arc;
use fxhash::FxHashMap;
use util::graph_cell::*;
use util::{InsertUnique, InsertGraphCell};
use compile::{TokenSpan, TokenValue};
use ast::var::Variable;
use super::*;

/// When auto grouping is on, you can only own
/// one instance of the collectable where
/// new awards increase the amount property.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AutoGrouping {
  None,
  /// Defaults to `None` if the type doesn't have a parent.
  Inherit,
  ByAmount,
}

/// This is a super type for any of its collectables.
#[derive(Debug)]
pub struct CollectableGroup<'a> {
  name: TokenValue<Arc<str>>,
  ast: GraphRefMut<'a, Ast<'a>>,
  //self_ref: GraphRef<'a, Self>,
  auto_grouping: AutoGrouping,
  parent: Option<GraphRef<'a, CollectableGroup<'a>>>,
  properties: FxHashMap<Arc<str>, GraphCell<Variable<'a>>>,
  collectables: FxHashMap<Arc<str>, ItemRefMut<'a, Collectable<'a>>>,
}

impl_name_traits!((<'a>) CollectableGroup (<'a>));
named_display!((<'a>) CollectableGroup (<'a>));

impl<'a> CollectableGroup<'a> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'a, Ast<'a>>) -> Self {
    CollectableGroup {
      name,
      ast,
      auto_grouping: AutoGrouping::Inherit,
      parent: None,
      properties: Default::default(),
      collectables: Default::default(),
    }
  }

  pub fn auto_grouping(&self) -> AutoGrouping {
    self.auto_grouping
  }

  pub fn set_auto_grouping(&mut self, ag: AutoGrouping) {
    self.auto_grouping = ag;
  }
}

impl<'a> SourceItem for CollectableGroup<'a> {
  fn source_name(&self) -> &TokenValue<Arc<str>> {
    &self.name
  }

  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    for c in self.collectables.values_mut() {
      c.resolve(&*self.ast.awake_ref())?;//.set_parent(self.self_ref)?;
    }
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> CastType<'a> for CollectableGroup<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::CollectableGroup;
}

impl<'a> CustomType<'a> for CollectableGroup<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::CollectableGroup
  }

  fn capabilities(&self) -> TypeCapability {
    TC_PROPERTIES
  }

  fn property(&self, name: &str) -> Option<GraphRef<'a, Variable<'a>>> {
    self.properties.get(name).map(|p| p.asleep())
  }

  fn super_type(&self) -> Option<GraphRef<'a, CustomType<'a>>> {
    None
  }
}

impl<'a> SubType<'a, CollectableGroup<'a>> for CollectableGroup<'a> {
  fn super_type(&self) -> Option<GraphRef<'a, CollectableGroup<'a>>> {
    self.parent
  }

  fn assign_super_type_internal(&mut self, super_type: GraphRef<'a, CollectableGroup<'a>>) {
    self.parent = Some(super_type);
  }
}

impl<'a> Owner<'a, Variable<'a>> for CollectableGroup<'a> {
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

impl<'a> RefMutOwner<'a, Collectable<'a>> for CollectableGroup<'a> {
  fn insert_ref_mut(&mut self, r: ItemRefMut<'a, Collectable<'a>>) -> Result<()> {
    self.collectables
      .insert_unique(r.source_name().value().clone(), r)
      .map_err(|(name, _)| ErrorKind::DuplicateDefinition(name, "collectable").into())
  }

  fn has_ref_mut(&self, name: &str) -> bool {
    self.collectables.contains_key(name)
  }
}

#[derive(Debug)]
pub struct Collectable<'a> {
  name: TokenValue<Arc<str>>,
  parent: Option<GraphRef<'a, CollectableGroup<'a>>>,
  auto_grouping: AutoGrouping,
  properties: FxHashMap<Arc<str>, GraphCell<Variable<'a>>>,
  // upgrades
  // redemptions
}

impl_name_traits!((<'a>)Collectable(<'a>));
named_display!((<'a>)Collectable(<'a>));

impl<'a> Collectable<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
  )
    -> Self
  {
    Collectable {
      name,
      parent: None,
      auto_grouping: AutoGrouping::Inherit,
      properties: Default::default(),
    }
  }

  pub fn auto_grouping(&self) -> AutoGrouping {
    self.auto_grouping
  }

  pub fn set_auto_grouping(&mut self, auto_grouping: AutoGrouping) {
    self.auto_grouping = auto_grouping;
  }
}

impl<'a> SourceItem for Collectable<'a> {
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

impl<'a> CastType<'a> for Collectable<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Collectable;
}

impl<'a> CustomType<'a> for Collectable<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Collectable
  }

  fn capabilities(&self) -> TypeCapability {
    TC_PROPERTIES | TC_OWNED
  }

  fn super_type(&self) -> Option<GraphRef<'a, CustomType<'a>>> {
    //self.parent.map(|p| &p)
    None
  }

  fn property(&self, name: &str) -> Option<GraphRef<'a, Variable<'a>>> {
    //self.properties.get(name)
    //  .or_else(|| self.parent.map(|p| p.properties.get(name)))
    None
  }
}

impl<'a> SubType<'a, CollectableGroup<'a>> for Collectable<'a> {
  fn super_type(&self) -> Option<GraphRef<'a, CollectableGroup<'a>>> {
    self.parent
  }

  fn assign_super_type_internal(&mut self, super_type: GraphRef<'a, CollectableGroup<'a>>) {
    self.parent = Some(super_type);
  }
}

impl<'a> Owner<'a, Variable<'a>> for Collectable<'a> {
  fn insert(&mut self, p: Variable<'a>) -> Result<GraphRefMut<'a, Variable<'a>>> {
    self.properties
      .insert_graph_cell(p.source_name().value().clone(), p)
      .map_err(|p| ErrorKind::DuplicateDefinition(
        p.source_name().value().clone(),
        "property"
      ).into())
  }

  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Variable<'a>>> {
    self.properties.get(name).map(|r| r.asleep_mut())
  }
}
