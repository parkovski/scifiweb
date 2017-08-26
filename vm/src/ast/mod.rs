use std::sync::{Arc, Weak};
use std::mem;

pub enum ItemRef<T> {
  /// A concrete item reference.
  Resolved(T),
  /// A not-yet-defined item reference by name.
  Placeholder(String),
  /// Tried to resolve the reference and failed.
  Invalid(String),
}

impl<T> ItemRef<T> {
  pub fn unwrap(self) -> T {
    match self {
      ItemRef::Resolved(container) => container,
      _ => panic!("Called ItemRef::unwrap() on unresolved value"),
    }
  }

  pub fn unwrap_invalid(self) -> String {
    match self {
      ItemRef::Invalid(s) => s,
      _ => panic!("Called ItemRef::unwrap_invalid() on non-invalid value"),
    }
  }

  pub fn is_resolved(&self) -> bool {
    match *self {
      ItemRef::Resolved(_) => true,
      _ => false,
    }
  }

  pub fn is_placeholder(&self) -> bool {
    match *self {
      ItemRef::Placeholder(_) => true,
      _ => false,
    }
  }

  pub fn is_invalid(&self) -> bool {
    match *self {
      ItemRef::Invalid(_) => true,
      _ => false,
    }
  }

  pub fn resolve<F>(&mut self, resolver: F) -> bool
  where
    F: FnOnce(&str) -> Option<T>
  {
    let (new_val, success) = if let ItemRef::Placeholder(ref mut name) = *self {
      match resolver(name) {
        Some(resolved) => (ItemRef::Resolved(resolved), true),
        None => (ItemRef::Invalid(mem::replace(name, String::new())), false),
      }
    } else {
      return self.is_resolved();
    };
    *self = new_val;
    success
  }
}

pub struct Ast {
  pub users: Vec<Arc<User>>,
  pub collectable_groups: Vec<Arc<CollectableGroup>>,
  pub maps: Vec<Arc<Map>>,
  pub events: Vec<Arc<Event>>,
  pub randoms: Vec<Arc<Random>>,
}

impl Ast {
  pub fn new() -> Self {
    Ast {
      users: Vec::new(),
      collectable_groups: Vec::new(),
      maps: Vec::new(),
      events: Vec::new(),
      randoms: Vec::new(),
    }
  }

  pub fn merge(&mut self, other: Ast) {
    self.users.extend(other.users);
    self.collectable_groups.extend(other.collectable_groups);
    self.maps.extend(other.maps);
    self.events.extend(other.events);
    self.randoms.extend(other.randoms);
  }
}

pub trait AstAdd<T> {
  fn add(&mut self, item: Arc<T>);
}

impl AstAdd<User> for Ast {
  fn add(&mut self, item: Arc<User>) {
    self.users.push(item);
  }
}

impl AstAdd<CollectableGroup> for Ast {
  fn add(&mut self, item: Arc<CollectableGroup>) {
    self.collectable_groups.push(item);
  }
}

impl AstAdd<Map> for Ast {
  fn add(&mut self, item: Arc<Map>) {
    self.maps.push(item);
  }
}

impl AstAdd<Event> for Ast {
  fn add(&mut self, item: Arc<Event>) {
    self.events.push(item);
  }
}

impl AstAdd<Random> for Ast {
  fn add(&mut self, item: Arc<Random>) {
    self.randoms.push(item);
  }
}

pub struct User {
  pub name: String,
  pub collectables: Vec<CollectableProperty>,
  pub properties: Vec<Variable>,
}

impl User {
  pub fn new(name: String) -> Self {
    User {
      name,
      collectables: Vec::new(),
      properties: Vec::new(),
    }
  }
}

/// A reference to an item that can be
/// either single or part of a group.
pub enum GrpRef<T, G> {
  Single(ItemRef<Arc<T>>),
  Group(ItemRef<Arc<G>>),
}

pub struct CollectableProperty {
  pub item: GrpRef<Collectable, CollectableGroup>,
  pub amount: Range,
}

/// All collectables are in a group,
/// but for the ones defined without the
/// group keyword, there is only one member
/// with the same name as the group.
pub struct CollectableGroup {
  pub name: String,
  pub has_amount: bool,
  pub properties: Vec<Arc<Variable>>,
  // TODO: subgroups
  pub collectables: Vec<Arc<Collectable>>,
}

impl CollectableGroup {
  pub fn new(name: String) -> Self {
    CollectableGroup {
      name,
      has_amount: false,
      properties: Vec::new(),
      collectables: Vec::new(),
    }
  }
}

impl AstAdd<Variable> for CollectableGroup {
  fn add(&mut self, item: Arc<Variable>) {
    self.properties.push(item);
  }
}

impl AstAdd<Collectable> for CollectableGroup {
  fn add(&mut self, item: Arc<Collectable>) {
    self.collectables.push(item);
  }
}

pub struct Collectable {
  pub name: String,
  pub group: ItemRef<Weak<CollectableGroup>>,
  pub upgrades: Vec<Upgrade>,
  pub redemptions: Vec<Redemption>,
}

impl Collectable {
  pub fn new(name: String, group: String) -> Self {
    Collectable {
      name,
      group: ItemRef::Placeholder(group),
      upgrades: Vec::new(),
      redemptions: Vec::new(),
    }
  }
}

pub struct Upgrade {
  pub cost: GrpRef<Collectable, CollectableGroup>,
  pub cost_amount: Expression,
  pub self_amount_range: Range,
}

pub enum Redemption {
  ForCurrency,
  ForCollectable {
    self_amount: u32,
    cost: GrpRef<Collectable, CollectableGroup>,
    cost_amount: u32,
  }
}

pub enum VarRef {
  Amount,
  Var(ItemRef<Arc<Variable>>),
}

pub struct Variable {
  pub name: String,
  pub ty: Type,
  pub initial_value: Option<Expression>,
}

impl Variable {
  pub fn new(name: String, ty: Type, initial_value: Option<Expression>) -> Self {
    Variable { name, ty, initial_value }
  }
}

pub struct Range {
  pub min: Option<i32>,
  pub max: Option<i32>,
}

impl Range {
  pub fn new(min: i32, max: i32) -> Self {
    Range { min: Some(min), max: Some(max) }
  }

  pub fn exact(amt: i32) -> Self {
    Range { min: Some(amt), max: Some(amt) }
  }

  pub fn with_min(min: i32) -> Self {
    Range { min: Some(min), max: None }
  }

  pub fn with_max(max: i32) -> Self {
    Range { min: None, max: Some(max) }
  }

  pub fn none() -> Self {
    Range { min: None, max: None }
  }
}

/// A map contains correlations
/// between a constant value of any key
/// property on an entity and one value
/// property with variable expressions,
/// and must have an exhaustive branch
/// (or = expr).
pub struct Map {
  pub key_type: Type,
  pub value_type: Type,
  pub value_property: VarRef,
  pub branches: Vec<MapBranch>,
  pub default_branch: Expression,
}

pub struct MapBranch {
  pub key_property: VarRef,
  pub key_value: Constant,
  pub value_expr: Expression,
}

/// Do we create a random assortment
/// of collectables or a random assortment
/// of groups that contain the same
/// collectable?
pub enum RandomDistribution {
  Individual,
  /// The range indicates how many
  /// groups to make. The number of items
  /// per group will vary based on the
  /// weights so no matter the group
  /// distribution settings, it won't be
  /// possible to get a bunch of something
  /// that's set to be super rare.
  Group(Range),
}

pub enum RandomList {
  Weighted(Vec<(f32, Type)>),
  Unweighted(Vec<Type>),
}

/// A Random is a collectable generator.
/// It takes an amount expression
pub struct Random {
  pub distribution: RandomDistribution,
  pub item_type: Type,
  pub amount: Range,
  pub items: RandomList,
}

pub struct Event {
  pub name: String,
  pub params: Vec<EventParam>,
  pub commands: Vec<Box<Command>>,
}

impl Event {
  pub fn new(name: String) -> Self {
    Event {
      name,
      params: Vec::new(),
      commands: Vec::new(),
    }
  }
}

pub struct EventParam {
  pub name: String,
  pub ty: Type,
}

pub enum RemoteEventTarget {
  GameServer,
  User,
}

pub struct RemoteEvent {
  pub target: RemoteEventTarget,
  pub params: Vec<EventParam>,
}

pub enum Type {
  Switch,
  Text { localized: bool },
  GameServer,
  Admin,
  DateTime,
  GameResult,
  Random(ItemRef<Arc<Random>>),
  User(ItemRef<Arc<User>>),
  Collectable(ItemRef<Arc<Collectable>>),
  CollectableGroup(ItemRef<Arc<CollectableGroup>>),
  Event(ItemRef<Arc<Event>>),
  Map(ItemRef<Arc<Map>>),
}

pub enum Constant {
  Switch(bool),
  Int(i32),
  Float(f32),
  Text(String),
}

pub enum Expression {
  Unary(UnaryExpression),
  Binary(BinaryExpression),
  Constant(Constant),
  ReadVar(VarRef),
  ReadProperty(VarRef, String),
  Command(Box<Command>),
}

pub struct UnaryExpression {
  pub op: UnaryOp,
  pub expr: Box<Expression>,
}

pub struct BinaryExpression {
  pub op: BinaryOp,
  pub left: Box<Expression>,
  pub right: Box<Expression>,
}

pub enum UnaryOp {
  Not,
  Negate,
}

pub enum BinaryOp {
  Add,
  Subtract,
  Multiply,
  Divide,
  Power,
  Modulo,
  Less,
  LessEqual,
  Greater,
  GreaterEqual,
  Equal,
  NotEqual,
}

pub struct CommandVisitor;
pub trait Command {
  fn visit(&self, visitor: &mut CommandVisitor);
}

pub struct AuthorizeCommand;
impl Command for AuthorizeCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct AssertCommand;
impl Command for AssertCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct AwardCommand;
impl Command for AwardCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct OptionCommand {
  pub options: Vec<Box<Command>>,
}
impl Command for OptionCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct SetCommand;
impl Command for SetCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct FindCommand;
impl Command for FindCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct TimerCommand;
impl Command for TimerCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct NotifyCommand;
impl Command for NotifyCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}

pub struct CostCommand;
impl Command for CostCommand {
  fn visit(&self, visitor: &mut CommandVisitor) {}
}
