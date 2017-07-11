#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupType {
  name: String,
}

impl GroupType {
  pub fn new(name: String) -> Self {
    GroupType {
      name,
    }
  }

  pub fn name(&self) -> &str {
    self.name.as_str()
  }
}