use std::iter::FromIterator;
use std::slice::Iter;
use std::ops::Deref;
use std::fmt::{self, Display, Debug};
use serde::{Serialize, Serializer};
use serde::ser::SerializeStruct;

/// A `Vec` type that maintains a simple sorting order:
/// items are either on the left or right. The order is
/// maintained by a mem swap and inc/dec of the split index.
/// This is more of an optimization of maintaining two
/// sets where items are frequently moved without allocation
/// than a regular Vec.
pub struct SplitVec<T> {
  /// The underlying `Vec`.
  vec: Vec<T>,
  /// The split index. All items with a smaller index
  /// are "left", and items with an equal or greater
  /// index are "right".
  split_index: usize,
}

impl<T> SplitVec<T> {
  pub fn new() -> Self {
    SplitVec { vec: Vec::new(), split_index: 0 }
  }

  pub fn with_capacity(capacity: usize) -> Self {
    SplitVec { vec: Vec::with_capacity(capacity), split_index: 0 }
  }

  pub fn left_from_vec(vec: Vec<T>) -> Self {
    let len = vec.len();
    SplitVec { vec, split_index: len }
  }

  pub fn right_from_vec(vec: Vec<T>) -> Self {
    SplitVec { vec, split_index: 0 }
  }

  pub fn push_left(&mut self, item: T) {
    let len = self.vec.len();
    self.vec.push(item);
    self.move_item(len, true);
  }

  pub fn push_right(&mut self, item: T) {
    self.vec.push(item);
  }

  /// Returns the new index of the item. The item that was at the
  /// new index is now at the old index.
  fn move_item(&mut self, index: usize, to_left: bool) -> usize {
    let split_index = self.split_index;
    if to_left == (index < split_index) {
      // The item is already on the correct side.
      index
    } else if to_left {
      if index > split_index {
        self.vec.swap(index, split_index);
      }
      self.split_index += 1;
      split_index
    } else {
      // If the split index is 0, all items will be
      // on the right already, so we can assume this
      // index is valid here.
      let new_split_index = split_index - 1;
      if index < new_split_index {
        self.vec.swap(index, new_split_index);
      }
      self.split_index -= 1;
      new_split_index
    }
  }

  pub fn move_left(&mut self, index: usize) -> usize {
    self.move_item(index, true)
  }

  pub fn move_right(&mut self, index: usize) -> usize {
    self.move_item(index, false)
  }

  pub fn split_index(&self) -> usize {
    self.split_index
  }

  pub fn is_left(&self, index: usize) -> bool {
    index < self.split_index
  }

  pub fn is_right(&self, index: usize) -> bool {
    index >= self.split_index
  }

  pub fn left_iter(&self) -> Iter<T> {
    let split_index = self.split_index;
    (&self.vec[0..split_index]).iter()
  }

  pub fn right_iter(&self) -> Iter<T> {
    let split_index = self.split_index;
    (&self.vec[split_index..]).iter()
  }

  pub fn into_vec(self) -> Vec<T> {
    self.vec
  }

  pub fn as_vec(&self) -> &Vec<T> {
    &self.vec
  }

  pub fn left_len(&self) -> usize {
    self.split_index
  }

  pub fn right_len(&self) -> usize {
    self.len() - self.split_index
  }
}

impl<T: Clone> Clone for SplitVec<T> {
  fn clone(&self) -> Self {
    SplitVec {
      vec: self.vec.clone(),
      split_index: self.split_index,
    }
  }
}

impl<T> Deref for SplitVec<T> {
  type Target = Vec<T>;
  fn deref(&self) -> &Vec<T> {
    &self.vec
  }
}

impl<A> FromIterator<A> for SplitVec<A> {
  fn from_iter<T>(iter: T) -> Self where T: IntoIterator<Item = A> {
    SplitVec { vec: FromIterator::from_iter(iter), split_index: 0 }
  }
}

impl<T: Debug> Debug for SplitVec<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "[")?;
    let len = self.vec.len();
    let split = self.split_index;
    for i in 0..len {
      if i == split {
        write!(f, " | ")?;
      } else if i > 0 {
        write!(f, ", ")?;
      }
      Debug::fmt(&self.vec[i], f)?;
    }
    if split == len {
      write!(f, " | ")?;
    }
    write!(f, "]")
  }
}

impl<T: Debug + Display> Display for SplitVec<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    Debug::fmt(self, f)
  }
}

impl<T: Serialize> Serialize for SplitVec<T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_struct("SplitVec", 2)?;
    state.serialize_field("split_index", &self.split_index)?;
    state.serialize_field("vec", &self.vec)?;
    state.end()
  }
}

#[cfg(test)]
mod test {
  use super::SplitVec;

  #[test]
  fn iterate_empty() {
    let v = SplitVec::<i32>::new();
    assert!(None == v.iter().next())
  }

  #[test]
  fn iterate_left_only() {
    let mut v = SplitVec::new();
    for n in 1..6 {
      v.push_left(n);
    }
    assert!(v.left_iter().len() == 5);
    assert!(v.right_iter().len() == 0);
    for (n, &m) in (1..6).zip(v.left_iter()) {
      assert!(n == m);
    }
  }

  #[test]
  fn iterate_right_only() {
    let mut v = SplitVec::new();
    for n in 1..6 {
      v.push_right(n);
    }
    assert!(v.left_iter().len() == 0);
    assert!(v.right_iter().len() == 5);
    for (n, &m) in (1..6).zip(v.right_iter()) {
      assert!(n == m);
    }
  }

  #[test]
  fn iterate_both() {
    let mut v = SplitVec::new();
    for n in 1..10 {
      if (n % 2) == 0 {
        v.push_left(n);
      } else {
        v.push_right(n);
      }
    }
    assert!(v.left_iter().len() == 4);
    assert!(v.right_iter().len() == 5);
    let mut all_left = v.clone();
    let mut all_right = v.clone();
    assert!(v.into_vec() == [2, 4, 6, 8, 5, 3, 7, 1, 9]);
    for _ in 0..9 {
      all_left.move_left(8);
    }
    for _ in 0..9 {
      all_right.move_right(0);
    }
    assert!(all_left.left_iter().len() == 9);
    assert!(all_left.right_iter().len() == 0);
    assert!(all_left.into_vec() == [2, 4, 6, 8, 9, 5, 3, 7, 1]);

    assert!(all_right.left_iter().len() == 0);
    assert!(all_right.right_iter().len() == 9);
    assert!(all_right.into_vec() == [4, 6, 8, 2, 5, 3, 7, 1, 9]);
  }
}