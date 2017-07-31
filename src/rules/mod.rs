use std::collections::HashMap;

pub mod collectable;
pub use self::collectable::Collectable;
pub mod config;
pub mod event;
pub use self::event::{Event, EventTarget};
pub mod group;
pub use self::group::GroupType;

pub struct RuleGraph<'a> {
  group_type_map: HashMap<String, GroupType>,
  collectable_map: HashMap<String, Collectable<'a>>,
  event_map: HashMap<String, Event>,
}

impl<'a> RuleGraph<'a> {
  pub fn new(
    group_type_map: HashMap<String, GroupType>,
    collectable_map: HashMap<String, Collectable<'a>>,
    event_map: HashMap<String, Event>,
  ) -> Self {
    RuleGraph {
      group_type_map,
      collectable_map,
      event_map,
    }
  }
}
