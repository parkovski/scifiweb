use std::sync::{Arc, Weak};
use std::ops::Deref;
use std::mem;

pub struct ItemRef<C: Deref + Clone> {
  /// A concrete item reference.
  Resolved(C),
  /// A not-yet-defined item reference by name.
  Placeholder(String),
  /// Tried to resolve the reference and failed.
  Invalid(String),
}

impl<C: Deref + Clone> ItemRef<C> {
  fn unwrap(&self) -> C {
    match *self {
      ItemRef::Resolved(ref container) => container.clone(),
      _ => panic!("Called ItemRef::unwrap() on unresolved value"),
    }
  }

  fn unwrap_invalid(&self) -> &str {
    match *self {
      ItemRef::Invalid(ref s) => s,
      _ => panic!("Called ItemRef::unwrap_invalid() on non-invalid value"),
    }
  }

  fn resolve<F>(&mut self, resolver: F) -> bool
  where
    F: FnOnce(&str) -> Option<C>
  {
    let (new_val, success) = if let ItemRef::Placeholder(ref mut name) = *self {
      match resolver(name) {
        Some(resolved) => (ItemRef::Resolved(resolved), true),
        None => (ItemRef::Invalid(mem::replace(name, String::new())), false),
      }
    };
    *self = new_val;
    success
  }
}

pub struct Include {
  pub filename: String,
}

pub struct User {
  pub name: String,
  pub collectables: Vec<CollectableProperty>,
  pub properties: Vec<Variable>,
}

pub enum CollectableRef {
  Single(ItemRef<Arc<Collectable>>),
  Group(ItemRef<Arc<CollectableGroup>>),
}

pub struct CollectableProperty {
  pub item: CollectableRef,
  pub amount: Range,
}

/// All collectables are in a group,
/// but for the ones defined without the
/// group keyword, there is only one member
/// with the same name as the group.
pub struct CollectableGroup {
  pub name: String,
  pub has_amount: bool,
  pub properties: Vec<Variable>,
  pub collectables: Vec<Collectable>,
}

pub struct Collectable {
  pub name: String,
  pub group: Weak<CollectableGroup>,
  pub upgrades: Vec<Upgrade>,
  pub redemptions: Vec<Redemption>,
}

pub struct Upgrade {
  pub cost: CollectableRef,
  pub cost_amount: Expression,
  pub self_amount_range: Range,
}

pub enum Redemption {
  ForCurrency,
  ForCollectable {
    self_amount: u32,
    cost: CollectableRef,
    cost_amount: u32,
  }
}

pub enum VarRef {
  Amount,
  ItemRef<Arc<Variable>>,
}

pub struct Variable {
  pub name: String,
  pub ty: Type,
  pub initial_value: Expression,
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
  pub params: Vec<EventParam>,
  pub commands: Vec<Box<Command>>,
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

pub enum Expression {
  Unary(UnaryExpression),
  Binary(BinaryExpression),
  IntConstant(i32),
  FloatConstant(f32),
  StringConstant(String),
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

struct CommandVisitor;
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