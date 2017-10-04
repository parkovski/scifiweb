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
pub struct CollectableGroup<'ast> {
  name: TokenValue<Arc<str>>,

  self_ref: Later<GraphRef<'ast, CollectableGroup<'ast>>>,

  auto_grouping: AutoGrouping,

  parent: Option<GraphRef<'ast, CollectableGroup<'ast>>>,

  scope: GraphCell<Scope<'ast>>,

  collectables: FxHashMap<Arc<str>, ItemRefMut<'ast, Collectable<'ast>>>,
  sub_groups: FxHashMap<Arc<str>, ItemRefMut<'ast, CollectableGroup<'ast>>>,

  upgrades: Option<Vec<Upgrade>>,
  redemptions: Option<Vec<Redemption>>,
}

impl<'ast> CollectableGroup<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    let cg = Ast::insert_cast_type(
      ast,
      CollectableGroup {
        name,
        self_ref: Later::new(),
        auto_grouping: AutoGrouping::Inherit,
        parent: None,
        scope: Scope::child(parent_scope, ScopeKind::TYPE | ScopeKind::RECURSIVE, span),
        collectables: Default::default(),
        sub_groups: Default::default(),
        upgrades: None,
        redemptions: None,
      }
    )?;
    Later::set(&mut cg.awake_mut().self_ref, cg.asleep_ref());
    Ok(cg)
  }

  pub fn auto_grouping(&self) -> AutoGrouping {
    self.auto_grouping
  }

  pub fn set_auto_grouping(&mut self, ag: AutoGrouping) {
    self.auto_grouping = ag;
  }

  pub fn insert_collectable_ref(&mut self, r: ItemRefMut<'ast, Collectable<'ast>>) -> Result<()> {
    self.collectables
      .insert_unique(r.name().value().clone(), r)
      .map_err(|(_, r)|
        ErrorKind::DuplicateDefinition(
          r.name().clone(), "collectable"
        ).into()
      )
  }

  pub fn insert_group_ref(&mut self, r: ItemRefMut<'ast, CollectableGroup<'ast>>) -> Result<()> {
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
  CollectableGroup<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('ast,)
);

impl<'ast> SourceItem for CollectableGroup<'ast> {
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

impl<'ast> CastType<'ast> for CollectableGroup<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::CollectableGroup;
}

impl<'ast> CustomType<'ast> for CollectableGroup<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::CollectableGroup
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::PROPERTIES | TypeCapability::INHERIT
  }

  fn property(&self, name: &str) -> Option<GraphRef<'ast, Variable<'ast>>> {
    self.scope.awake().find(name)
  }

  fn is_sub_type_of(&self, _ty: &CustomType<'ast>) -> bool {
    false
  }
}

impl<'ast> SubType<'ast, CollectableGroup<'ast>> for CollectableGroup<'ast> {
  fn super_type(&self) -> Option<GraphRef<'ast, CollectableGroup<'ast>>> {
    self.parent
  }

  fn assign_super_type_internal(&mut self, super_type: GraphRef<'ast, CollectableGroup<'ast>>) {
    self.parent = Some(super_type);
  }
}

#[derive(Debug, Serialize)]
pub struct Collectable<'ast> {
  name: TokenValue<Arc<str>>,
  parent: Option<GraphRef<'ast, CollectableGroup<'ast>>>,
  auto_grouping: AutoGrouping,
  scope: GraphCell<Scope<'ast>>,
  upgrades: Option<Vec<Upgrade>>,
  redemptions: Option<Vec<Redemption>>,
}

impl<'ast> Collectable<'ast> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    ast: GraphRefMut<'ast, Ast<'ast>>,
  )
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    Ast::insert_cast_type(ast, Collectable {
      name,
      parent: None,
      auto_grouping: AutoGrouping::Inherit,
      scope: Scope::child(parent_scope, ScopeKind::TYPE | ScopeKind::RECURSIVE, span),
      upgrades: None,
      redemptions: None,
    })
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
  Collectable<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('ast,)
);

impl<'ast> SourceItem for Collectable<'ast> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    // TODO: This may not resolve super types, depending on order.
    // Need to change the way those are set, with a placeholder type.
    self.scope.awake_mut().resolve()
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'ast> CastType<'ast> for Collectable<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::Collectable;
}

impl<'ast> CustomType<'ast> for Collectable<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::Collectable
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::PROPERTIES | TypeCapability::OWNED | TypeCapability::INHERIT
  }

  fn is_sub_type_of(&self, _ty: &CustomType<'ast>) -> bool {
    false
  }

  fn property(&self, _name: &str) -> Option<GraphRef<'ast, Variable<'ast>>> {
    //self.properties.get(name)
    //  .or_else(|| self.parent.map(|p| p.properties.get(name)))
    None
  }
}

impl<'ast> SubType<'ast, CollectableGroup<'ast>> for Collectable<'ast> {
  fn super_type(&self) -> Option<GraphRef<'ast, CollectableGroup<'ast>>> {
    self.parent
  }

  fn assign_super_type_internal(&mut self, super_type: GraphRef<'ast, CollectableGroup<'ast>>) {
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
