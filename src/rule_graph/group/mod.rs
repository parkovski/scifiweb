use std;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::error::Error;
use std::fmt;

use super::profile::Profile;

#[derive(Debug)]
pub enum GroupAddError {
  AddType { description: String },
  TypeMissing { description: String },
  AddGroup { description: String },
}

impl GroupAddError {
  pub fn new_type_error<'a>(name: &'a String) -> Self {
    GroupAddError::AddType {
      description: format!("Group type '{}' already exists", &name),
    }
  }

  pub fn new_type_missing_error<'a>(name: &'a String) -> Self {
    GroupAddError::TypeMissing {
      description: format!("Group type '{}' does not exist", &name),
    }
  }

  pub fn new_group_error<'a>(name: &'a String) -> Self {
    GroupAddError::AddGroup {
      description: format!("Group '{}' already exists", &name),
    }
  }
}

impl fmt::Display for GroupAddError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    write!(f, "{}", self.description())
  }
}

impl Error for GroupAddError {
  fn description(&self) -> &str {
    match self {
      &GroupAddError::AddType { description: ref desc } => desc,
      &GroupAddError::TypeMissing { description: ref desc } => desc,
      &GroupAddError::AddGroup { description: ref desc } => desc,
    }.as_str()
  }
}

pub struct Group<'a> {
  name: String,
  group_type: &'a String,
  members: Vec<*const Profile<'a>>,
}

impl<'a> Group<'a> {
  pub fn new(name: String, group_type: &'a String) -> Self {
    Group {
      name,
      group_type,
      members: Vec::new(),
    }
  }

  pub fn get_name(&'a self) -> &'a String {
    &self.name
  }

  pub(in self) fn set_name(&mut self, name: String) {
    self.name = name;
  }

  pub fn get_type(&self) -> &'a String {
    self.group_type
  }

  pub(in self) fn set_type(&mut self, group_type: &'a String) {
    self.group_type = group_type;
  }

  pub fn members_iter(&'a self) -> Box<std::iter::Iterator<Item = &'a Profile<'a>> + 'a> {
    Box::new(self.members.iter().map(|m: &*const Profile<'a>| -> &'a Profile<'a> { unsafe { m.as_ref::<'a>().unwrap() } }))
  }

  pub fn add_member(&mut self, member: &'a Profile<'a>) {
    self.members.push(member as *const _);
  }
}

pub struct GroupMap<'a> {
  pub groups_by_name: HashMap<String, Group<'a>>,
  pub groups_by_type: HashMap<String, Vec<&'a Group<'a>>>,
}

impl<'a> GroupMap<'a> {
  pub fn new() -> Self {
    GroupMap {
      groups_by_name: HashMap::new(),
      groups_by_type: HashMap::new(),
    }
  }

  pub fn add_group_type(&mut self, group_type: String) -> Result<(), GroupAddError> {
    match self.groups_by_type.entry(group_type.clone()) {
      Entry::Occupied(_) => Err(GroupAddError::new_type_error(&group_type)),
      Entry::Vacant(e) => {
        e.insert(Vec::new());
        Ok(())
      }
    }
  }
}