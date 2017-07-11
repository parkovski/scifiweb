use std::fmt;

use rules;

pub mod access;

mod collectable;
pub use self::collectable::Collectable;

mod event;
pub use self::event::Event;

mod group;
pub use self::group::Group;

pub mod mailbox;

mod notification;

mod profile;
pub use self::profile::{ Profile, ProfileId };

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Target<'a> {
  Global,
  ProfileId(u64),
  GroupId(u64),
  GroupType(&'a rules::GroupType),
}

impl<'a> fmt::Display for Target<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Target::Global => write!(f, "Global"),
      &Target::ProfileId(id) => write!(f, "Profile {}", id),
      &Target::GroupId(id) => write!(f, "Group {}", id),
      &Target::GroupType(ty) => write!(f, "Group type {}", &ty.name()),
    }
  }
}

pub struct InstanceGraph<'a> {
  rules: &'a rules::RuleGraph<'a>,
}