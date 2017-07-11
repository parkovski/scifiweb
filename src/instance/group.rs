use rules;

use super::Profile;

pub struct Group<'a> {
  rules: &'a rules::GroupType,
  id: u64,
  name: String,
  member_ids: Vec<u64>,
  members: Option<Vec<&'a Profile>>,
  owner_id: u64,
  owner: Option<&'a Profile>,
}