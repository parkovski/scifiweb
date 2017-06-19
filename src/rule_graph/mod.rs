use std::collections::HashMap;
use core::slice;
use core::iter as core_iter;

pub mod collectable;
pub mod config;
pub mod event;
pub mod group;
pub mod profile;

pub struct RuleGraph<'a> {
  group_map: group::GroupMap<'a>,
  collectable_map: HashMap<String, collectable::Collectable<'a>>,
  event_map: HashMap<String, event::Event<'a>>,
}

impl<'a> RuleGraph<'a> {
  pub fn new(
    group_map: group::GroupMap<'a>,
    collectable_map: HashMap<String, collectable::Collectable<'a>>,
    event_map: HashMap<String, event::Event<'a>>
  ) -> Self
  {
    RuleGraph {
      group_map,
      collectable_map,
      event_map,
    }
  }

  pub fn get_group_by_name(&self, name: &String) -> Option<&group::Group<'a>> {
    self.group_map.groups_by_name.get(name)
  }

  pub fn get_groups_by_type(&self, group_type: &String)
    -> Option<
      core_iter::Map<
        slice::Iter<&'a group::Group<'a>>,
        fn(&&'a group::Group<'a>) -> &'a group::Group<'a>
      >
    >
  {
    fn double_to_single_ref<'b>(g: &&'b group::Group<'b>) -> &'b group::Group<'b> {
      *g
    }
    self.group_map.groups_by_type.get(group_type).map(|v| v.iter().map(
        double_to_single_ref as fn(&&'a group::Group<'a>) -> &'a group::Group<'a>
    ))
  }

  pub fn get_collectable_by_name(&self, name: &String) -> Option<&collectable::Collectable<'a>> {
    self.collectable_map.get(name)
  }

  pub fn get_event_by_name(&self, name: &String) -> Option<&event::Event<'a>> {
    self.event_map.get(name)
  }
}