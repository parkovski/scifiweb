use std::time::Duration;
use super::group::GroupType;

mod action;

pub use self::action::Action;

pub enum EventTarget {
  Global,
  Profile,
  Group,
  GroupType(*const GroupType),
}

pub struct Event {
  pub name: String,
  pub target: EventTarget,
  pub duration: Duration,
  pub action: Action,
}
