#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupType {
  name: Box<str>,
}

impl GroupType {
  pub fn new(name: String) -> Self {
    GroupType {
      name: name.into_boxed_str(),
    }
  }

  pub fn name(&self) -> &str {
    &self.name
  }
}
