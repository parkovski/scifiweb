use std::sync::Arc;
use fxhash::FxHashMap;
use util::graph_cell::*;
use util::later::Later;
use util::{InsertUnique};
use compile::{TokenSpan, TokenValue};
use ast::var::Variable;
use super::*;

/// When auto grouping is on, you can only own
/// one instance of the collectable where
/// new awards increase the amount property.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum AutoGrouping {
  None,
  /// Defaults to `None` if the type doesn't have a parent.
  Inherit,
  ByAmount,
}

/// This is a super type for any of its collectables.
#[derive(Debug, Serialize)]
pub struct CollectableGroup<'a> {
  name: TokenValue<Arc<str>>,

  self_ref: Later<GraphRef<'a, CollectableGroup<'a>>>,

  auto_grouping: AutoGrouping,

  parent: Option<GraphRef<'a, CollectableGroup<'a>>>,

  scope: GraphCell<Scope<'a>>,

  collectables: FxHashMap<Arc<str>, ItemRefMut<'a, Collectable<'a>>>,
  sub_groups: FxHashMap<Arc<str>, ItemRefMut<'a, CollectableGroup<'a>>>,

  upgrades: Option<Vec<Upgrade>>,
  redemptions: Option<Vec<Redemption>>,
}

impl<'a> CollectableGroup<'a> {
  pub fn new(name: TokenValue<Arc<str>>, parent_scope: GraphRef<'a, Scope<'a>>) -> Self {
    let span = name.span().clone();
    CollectableGroup {
      name,
      self_ref: Later::new(),
      auto_grouping: AutoGrouping::Inherit,
      parent: None,
      scope: Scope::child(parent_scope, span),
      collectables: Default::default(),
      sub_groups: Default::default(),
      upgrades: None,
      redemptions: None,
    }
  }

  pub fn auto_grouping(&self) -> AutoGrouping {
    self.auto_grouping
  }

  pub fn set_auto_grouping(&mut self, ag: AutoGrouping) {
    self.auto_grouping = ag;
  }

  pub fn insert_collectable_ref(&mut self, r: ItemRefMut<'a, Collectable<'a>>) -> Result<()> {
    self.collectables
      .insert_unique(r.name().value().clone(), r)
      .map_err(|(_, r)|
        ErrorKind::DuplicateDefinition(
          r.name().clone(), "collectable"
        ).into()
      )
  }

  pub fn insert_group_ref(&mut self, r: ItemRefMut<'a, CollectableGroup<'a>>) -> Result<()> {
    self.sub_groups
      .insert_unique(r.name().value().clone(), r)
      .map_err(|(_, r)|
        ErrorKind::DuplicateDefinition(
          r.name().clone(), "collectable group"
        ).into()
      )
  }

  pub fn insert_upgrades(&mut self, upgrades: Vec<Upgrade>) {
    self.upgrades = Some(upgrades);
  }

  pub fn insert_redemptions(&mut self, redemptions: Vec<Redemption>) {
    self.redemptions = Some(redemptions);
  }
}

type_macros!(
  CollectableGroup<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for CollectableGroup<'a> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    for g in self.sub_groups.values_mut() {
      g.resolve()?;
      let g = g.unwrap();
      g.awake_mut().set_super_type(*self.self_ref)?;
    }
    for c in self.collectables.values_mut() {
      c.resolve()?;
      let c = c.unwrap();
      c.awake_mut().set_super_type(*self.self_ref)?;
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
  fn init_cyclic(
    &mut self,
    self_ref: GraphRef<'a, Self>,
    _ast_ref: GraphRef<'a, Ast<'a>>
  ) where Self: Sized
  {
    self.self_ref.set(self_ref);
  }

  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::CollectableGroup
  }

  fn capabilities(&self) -> TypeCapability {
    TC_PROPERTIES | TC_INHERIT
  }

  fn property(&self, name: &str) -> Option<GraphRef<'a, Variable<'a>>> {
    self.scope.awake().find(name)
  }

  fn is_sub_type_of(&self, _ty: &CustomType<'a>) -> bool {
    false
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

#[derive(Debug, Serialize)]
pub struct Collectable<'a> {
  name: TokenValue<Arc<str>>,
  parent: Option<GraphRef<'a, CollectableGroup<'a>>>,
  auto_grouping: AutoGrouping,
  scope: GraphCell<Scope<'a>>,
  upgrades: Option<Vec<Upgrade>>,
  redemptions: Option<Vec<Redemption>>,
}

impl<'a> Collectable<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    parent_scope: GraphRef<'a, Scope<'a>>,
  )
    -> Self
  {
    let span = name.span().clone();
    Collectable {
      name,
      parent: None,
      auto_grouping: AutoGrouping::Inherit,
      scope: Scope::child(parent_scope, span),
      upgrades: None,
      redemptions: None,
    }
  }

  pub fn auto_grouping(&self) -> AutoGrouping {
    self.auto_grouping
  }

  pub fn set_auto_grouping(&mut self, auto_grouping: AutoGrouping) {
    self.auto_grouping = auto_grouping;
  }

  pub fn insert_upgrades(&mut self, upgrades: Vec<Upgrade>) {
    self.upgrades = Some(upgrades);
  }

  pub fn insert_redemptions(&mut self, redemptions: Vec<Redemption>) {
    self.redemptions = Some(redemptions);
  }
}

type_macros!(
  Collectable<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for Collectable<'a> {
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

impl<'a> CastType<'a> for Collectable<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Collectable;
}

impl<'a> CustomType<'a> for Collectable<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Collectable
  }

  fn capabilities(&self) -> TypeCapability {
    TC_PROPERTIES | TC_OWNED | TC_INHERIT
  }

  fn is_sub_type_of(&self, _ty: &CustomType<'a>) -> bool {
    false
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

#[derive(Debug, Serialize)]
pub struct Upgrade {
  level: u32,
}

impl Upgrade {
  pub fn new(level: u32) -> Self {
    Upgrade { level }
  }
}

#[derive(Debug, Serialize)]
pub struct Redemption {
  amount: u32,
}

impl Redemption {
  pub fn new(amount: u32) -> Self {
    Redemption { amount }
  }
}
