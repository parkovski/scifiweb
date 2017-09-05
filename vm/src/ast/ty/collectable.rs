use std::sync::Arc;
use fxhash::FxHashMap;
use util::graph_cell::*;
use util::{InsertUnique, InsertGraphCell};
use compile::{TokenSpan, TokenValue};
use ast::var::Property;
use ast::errors::*;
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
  auto_grouping: AutoGrouping,
  properties: FxHashMap<Arc<str>, GraphCell<Property<'a>>>,
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
/*
  pub fn add_property(&mut self, prop: Property<'a>) -> Result<()> {
    self.properties
      .insert_unique(prop.source_name().value().clone(), prop)
      .map_err(|(name, p)|
        ErrorKind::DuplicateDefinition(name, "property").into()
      )
  }

/*
  pub fn collectable(&self, name: &str) -> Option<&ItemRef<'a, Collectable<'a, R>, Ast<R>>> {
    self.collectables.get(name)
  }
  */

  pub fn add_collectable(
    &mut self,
    c: ItemRef<'a, Collectable<'a, R>, Ast<'a, R>>
  ) -> Result<()>
  {
    self.properties
      .insert_unique(c.source_name().value().clone(), c)
      .map_err(|(name, c)|
        ErrorKind::DuplicateDefinition(name, "collectable").into()
      )
  }
*/
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
      c.resolve(&*self.ast.awake_ref())?;
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

  fn property(&self, name: &str) -> Option<GraphRef<'a, Property<'a>>> {
    //self.properties.get(name)
    None
  }

  fn super_type(&self) -> Option<GraphRef<'a, CustomType<'a>>> {
    None
  }
}

impl<'a> Owner<'a, Property<'a>> for CollectableGroup<'a> {
  fn insert(&mut self, p: Property<'a>) -> Result<GraphRefMut<'a, Property<'a>>> {
    self.properties
      .insert_graph_cell(p.source_name().value().clone(), p)
      .map_err(|p| ErrorKind::DuplicateDefinition(
        p.source_name().value().clone(),
        "property"
      ).into())
  }

  fn find(&self, name: &str) -> Option<GraphRefMut<'a, Property<'a>>> {
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
  properties: FxHashMap<Arc<str>, GraphCell<Property<'a>>>,
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
/*
  pub fn set_parent(&mut self, parent: ItemRef<'a, CollectableGroup<'a>>) -> Result<()> {
    if let Some(ref p) = self.parent {
      if p.name() == parent.name() {
        return Ok(())
      } else {
        return Err(ErrorKind::ConflictingSuperType(
          self.name.value().clone(),
          p.source_name().value().clone(),
          parent.source_name().value().clone(),
        ).into());
      }
    }
    self.parent = Some(parent);
    Ok(())
  }*/
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
    TC_PROPERTIES
  }

  fn super_type(&self) -> Option<GraphRef<'a, CustomType<'a>>> {
    //self.parent.map(|p| &p)
    None
  }

  fn property(&self, name: &str) -> Option<GraphRef<'a, Property<'a>>> {
    //self.properties.get(name)
    //  .or_else(|| self.parent.map(|p| p.properties.get(name)))
    None
  }
}

impl<'a> Owner<'a, Property<'a>> for Collectable<'a> {
  fn insert(&mut self, p: Property<'a>) -> Result<GraphRefMut<'a, Property<'a>>> {
    self.properties
      .insert_graph_cell(p.source_name().value().clone(), p)
      .map_err(|p| ErrorKind::DuplicateDefinition(
        p.source_name().value().clone(),
        "property"
      ).into())
  }

  fn find(&self, name: &str) -> Option<GraphRefMut<'a, Property<'a>>> {
    self.properties.get(name).map(|r| r.asleep_mut())
  }
}
