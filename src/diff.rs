use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
  None,
  Single(String, String),
  SingleNested(String, Box<Change>),
  Nested(Changeset),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changeset {
  changes: HashMap<String, Change>,
}

impl Changeset {
  pub fn new() -> Self {
    Changeset { changes: HashMap::new() }
  }

  pub fn add_field<T: Diff>(&mut self, field_name: &str, old_value: &T, new_value: &T) {
    self.add_change(field_name, old_value.diff(new_value));
  }

  pub fn add_change(&mut self, field_name: &str, change: Change) {
    if change == Change::None {
      return;
    }
    if self.changes.insert(field_name.to_owned(), change).is_some() {
      panic!("Duplicate field inserted for changeset: {}", field_name);
    }
  }

  pub fn is_empty(&self) -> bool {
    self.changes.is_empty()
  }

  pub fn into_change(mut self) -> Change {
    match self.changes.len() {
      0 => Change::None,
      1 => {
        let (field, change) = self.changes.drain().next().unwrap();
        Change::SingleNested(field, Box::new(change))
      }
      _ => Change::Nested(self),
    }
  }
}

pub trait Diff {
  fn diff(&self, new_value: &Self) -> Change;
}

impl<T: ToString> Diff for T {
  fn diff(&self, new_value: &T) -> Change {
    let old_string_value = self.to_string();
    let new_string_value = new_value.to_string();
    if old_string_value == new_string_value {
      Change::None
    } else {
      Change::Single(old_string_value, new_string_value)
    }
  }
}

#[macro_export]
macro_rules! impl_diff_for {
  ($type:ty) => (
    impl $crate::diff::Diff for $type {
      fn diff(&self, new_value: &Self) -> $crate::diff::Change {
        if self == new_value {
          $crate::diff::Change::None
        } else {
          $crate::diff::Change::Single(self.to_string(), other.to_string())
        }
      }
    }
  );
  ($type:ty, $field:ident) => (
    impl $crate::diff::Diff for $type {
      fn diff(&self, new_value: &Self) -> $crate::diff::Change {
        if self.$field == new_value.$field {
          $crate::diff::Change::None
        } else {
          $crate::diff::Change::SingleNested(
            stringify!($field).to_owned(),
            Box::new($crate::diff::Change::Single(self.$field.to_string(), new_value.$field.to_string()))
          )
        }
      }
    }
  );
  ($type:ty, $($field:ident),+) => (
    impl $crate::diff::Diff for $type {
      fn diff(&self, new_value: &Self) -> $crate::diff::Change {
        let mut changeset = $crate::diff::Changeset::new();
        $(changeset.add(stringify!($field), &self.$field, &new_value.$field);)+
        changeset.into_change()
      }
    }
  );
}
