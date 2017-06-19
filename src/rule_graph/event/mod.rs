use std::time::{Instant, Duration};

use either::Either;
use super::profile::Profile;
use super::group::Group;

mod action;
pub use self::action::Action;

pub struct Event<'a> {
  pub name: String,
  /// None is a global event, otherwise it is specific to either a player or a group.
  pub target: Option<Either<&'a Profile<'a>, &'a Group<'a>>>,
  pub duration: Duration,
  pub action: Action,
}

pub struct EventInstance<'a> {
  pub event: &'a Event<'a>,
  pub start_instant: Instant,
}