use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut};
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::fmt::{self, Display, Debug};
use serde::ser::{Serialize, Serializer, SerializeTupleStruct};

const EMPTY_ERROR: &'static str = "Accessed Later value before initialization";

/// A value that we guarantee to initialize before we use it,
/// but that starts out uninitialized.
pub struct Later<T> {
  value: Option<T>,
}

impl<T> Later<T> {
  pub fn new() -> Self {
    Later { value: None }
  }

  pub fn set(&mut self, value: T) {
    self.value = Some(value);
  }

  pub fn is_set(&self) -> bool {
    self.value.is_some()
  }
}

impl<T> Default for Later<T> {
  fn default() -> Self {
    Later::new()
  }
}

impl<T: Debug> Debug for Later<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self.value {
      Some(ref t) => t.fmt(f),
      None => f.write_str("Later(empty!)"),
    }
  }
}

impl<T: Display> Display for Later<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.value.as_ref().expect(EMPTY_ERROR).fmt(f)
  }
}

impl<T: PartialEq> PartialEq for Later<T> {
  fn eq(&self, other: &Self) -> bool {
    self.value.as_ref().expect(EMPTY_ERROR).eq(other.value.as_ref().expect(EMPTY_ERROR))
  }
}

impl<T: Eq> Eq for Later<T> {}

impl<T: PartialOrd> PartialOrd for Later<T> {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.value.as_ref().expect(EMPTY_ERROR).partial_cmp(other.value.as_ref().expect(EMPTY_ERROR))
  }
}

impl<T: Ord> Ord for Later<T> {
  fn cmp(&self, other: &Later<T>) -> Ordering {
    self.value.as_ref().expect(EMPTY_ERROR).cmp(other.value.as_ref().expect(EMPTY_ERROR))
  }
}

impl<T: Clone> Clone for Later<T> {
  fn clone(&self) -> Self {
    Later { value: self.value.clone() }
  }
}

impl<T: Copy> Copy for Later<T> {}

impl<T: Hash> Hash for Later<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.value.as_ref().expect(EMPTY_ERROR).hash(state)
  }
}

impl<T> Borrow<T> for Later<T> {
  fn borrow(&self) -> &T {
    self.value.as_ref().expect(EMPTY_ERROR)
  }
}

impl<T> BorrowMut<T> for Later<T> {
  fn borrow_mut(&mut self) -> &mut T {
    self.value.as_mut().expect(EMPTY_ERROR)
  }
}

impl<T> Deref for Later<T> {
  type Target = T;
  fn deref(&self) -> &T {
    self.value.as_ref().expect(EMPTY_ERROR)
  }
}

impl<T> DerefMut for Later<T> {
  fn deref_mut(&mut self) -> &mut T {
    self.value.as_mut().expect(EMPTY_ERROR)
  }
}

impl<T: Serialize> Serialize for Later<T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_tuple_struct("Later", 1)?;
    state.serialize_field(&self.value)?;
    state.end()
  }
}
