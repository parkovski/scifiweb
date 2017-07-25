use std::fmt;
use std::str::FromStr;
use rules;
use util::error::FormatError;

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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
  Global,
  ProfileId(u64),
  GroupId(u64),
  GroupType(rules::GroupType),
}

impl Target {
  fn format_error() -> FormatError {
    FormatError::new("'global' or 'target type:id' (pid, gid, gty)")
  }
}

impl fmt::Display for Target {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Target::Global => write!(f, "Global"),
      &Target::ProfileId(id) => write!(f, "Profile {}", id),
      &Target::GroupId(id) => write!(f, "Group {}", id),
      &Target::GroupType(ref ty) => write!(f, "Group type {}", ty.name()),
    }
  }
}

impl FromStr for Target {
  type Err = FormatError;
  fn from_str(s: &str) -> Result<Self, FormatError> {
    if s == "global" {
      Ok(Target::Global)
    } else if let Some(split) = s.find(':') {
      match &s[0..split] {
        "pid" => s[split + 1..].parse::<u64>().map(|id| Target::ProfileId(id)).map_err(|_| Self::format_error()),
        "gid" => s[split + 1..].parse::<u64>().map(|id| Target::GroupId(id)).map_err(|_| Self::format_error()),
        "gty" => Ok(Target::GroupType(rules::GroupType::new(s[split + 1..].to_owned()))),
        _ => Err(Self::format_error())
      }
    } else {
      Err(Self::format_error())
    }
  }
}

pub struct InstanceGraph<'a> {
  rules: &'a rules::RuleGraph<'a>,
}