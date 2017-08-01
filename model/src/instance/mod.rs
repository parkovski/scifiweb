use std::fmt;
use std::str::FromStr;
use rules;
use rules::error::FormatError;

pub mod collectable;
pub mod event;
pub mod group;
pub mod messaging;
pub mod notification;
pub mod profile;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Target {
  Global,
  ProfileId(u64),
  GroupId(u64),
  GroupType(rules::group::GroupType),
}

impl Target {
  fn format_error() -> FormatError {
    FormatError::new("'global' or 'target type:id' (pid, gid, gty)")
  }
}

impl fmt::Display for Target {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Target::Global => write!(f, "Global"),
      Target::ProfileId(id) => write!(f, "Profile {}", id),
      Target::GroupId(id) => write!(f, "Group {}", id),
      Target::GroupType(ref ty) => write!(f, "Group type {}", ty.name()),
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
        "pid" => s[split + 1..]
          .parse::<u64>()
          .map(Target::ProfileId)
          .map_err(|_| Self::format_error()),
        "gid" => s[split + 1..]
          .parse::<u64>()
          .map(Target::GroupId)
          .map_err(|_| Self::format_error()),
        "gty" => Ok(Target::GroupType(
          rules::group::GroupType::new(s[split + 1..].to_owned()),
        )),
        _ => Err(Self::format_error()),
      }
    } else {
      Err(Self::format_error())
    }
  }
}

pub struct InstanceGraph<'a> {
  rules: &'a rules::RuleGraph<'a>,
}
