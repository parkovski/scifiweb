use std::cell::{Cell, UnsafeCell};
use std::ops::CoerceUnsized;
use std::marker::Unsize;
use std::ops::{Deref, DerefMut};
use std::hash::{Hash, Hasher};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::mem;

// =====

const WRITING: usize = !0usize;

fn acquire_for_read(borrow_count: &Cell<usize>) {
  let c = borrow_count.get();
  if c == WRITING {
    panic!("Can't take immutable awake borrow while mutable borrow is active");
  }
  borrow_count.set(c + 1);
}

fn acquire_for_write(borrow_count: &Cell<usize>) {
  let c = borrow_count.get();
  if c > 0 {
    panic!("Can't take mutable borrow while another borrow is active");
  }
  borrow_count.set(WRITING);
}

pub struct GraphCell<T: ?Sized> {
  borrow_count: Cell<usize>,
  data: UnsafeCell<T>,
}

impl<T> GraphCell<T> {
  pub fn new(data: T) -> Self {
    GraphCell {
      borrow_count: Cell::new(0),
      data: UnsafeCell::new(data),
    }
  }
}

impl<'a, T> GraphCell<T>
where
  Self: 'a,
  T: ?Sized + 'a,
{
  pub fn asleep(&'a self) -> GraphRef<'a, T> {
    GraphRef { data: self.data.get(), borrow_count: &self.borrow_count }
  }

  pub fn asleep_mut(&'a self) -> GraphRefMut<'a, T> {
    GraphRefMut { data: self.data.get(), borrow_count: &self.borrow_count }
  }

  pub fn awake(&'a self) -> GraphRefAwake<'a, T> {
    acquire_for_read(&self.borrow_count);
    GraphRefAwake {
      data: unsafe { &*self.data.get() },
      borrow_count: &self.borrow_count
    }
  }

  pub fn awake_mut(&'a self) -> GraphRefAwakeMut<'a, T> {
    acquire_for_write(&self.borrow_count);
    GraphRefAwakeMut {
      data: unsafe { &mut *self.data.get() },
      borrow_count: &self.borrow_count
    }
  }
}

impl<T: Debug + ?Sized> Debug for GraphCell<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    self.awake().fmt(f)
  }
}

impl<T: Clone> Clone for GraphCell<T> {
  fn clone(&self) -> Self {
    GraphCell::new(self.awake().clone())
  }
}

impl<T> From<T> for GraphCell<T> {
  fn from(data: T) -> Self {
    GraphCell::new(data)
  }
}

impl<T: Default> Default for GraphCell<T> {
  fn default() -> Self {
    GraphCell::new(T::default())
  }
}

unsafe impl<T: Send + ?Sized> Send for GraphCell<T> {}

impl<T: PartialEq<T> + ?Sized> PartialEq for GraphCell<T> {
  fn eq(&self, other: &Self) -> bool {
    self.awake().eq(&other.awake())
  }
}

impl<T: Eq + ?Sized> Eq for GraphCell<T> {}

impl<T: PartialOrd<T> + ?Sized> PartialOrd for GraphCell<T> {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    self.awake().partial_cmp(&other.awake())
  }
}

impl<T: Ord + ?Sized> Ord for GraphCell<T> {
  fn cmp(&self, other: &Self) -> Ordering {
    self.awake().cmp(&other.awake())
  }
}

impl<T: Hash + ?Sized> Hash for GraphCell<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.awake().hash(state)
  }
}

impl<T: CoerceUnsized<U>, U> CoerceUnsized<GraphCell<U>> for GraphCell<T>
{}

// =====

#[derive(Copy, Clone)]
pub struct GraphRef<'a, T: ?Sized + 'a> {
  data: *const T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRef<'a, T> {
  pub fn awake(&self) -> GraphRefAwake<'a, T> {
    acquire_for_read(self.borrow_count);
    GraphRefAwake {
      data: unsafe { &*self.data },
      borrow_count: self.borrow_count,
    }
  }

  pub fn map<F, U>(&self, map_fn: F) -> GraphRef<'a, U>
  where
    F: FnOnce(&'a T) -> &'a U,
    U: 'a,
  {
    acquire_for_read(self.borrow_count);
    let ref_data: &'a T = unsafe { &*self.data };
    let new_ref_data = map_fn(ref_data);
    let new_data = new_ref_data as *const _;
    self.borrow_count.set(self.borrow_count.get() - 1);
    GraphRef { data: new_data, borrow_count: self.borrow_count }
  }
}

impl<'a, T, U> CoerceUnsized<GraphRef<'a, U>> for GraphRef<'a, T>
where
  T: Unsize<U> + ?Sized + 'a,
  U: ?Sized + 'a,
{}

// =====

#[derive(Copy, Clone)]
pub struct GraphRefMut<'a, T: ?Sized + 'a> {
  data: *mut T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRefMut<'a, T> {
  pub fn asleep_ref(&self) -> GraphRef<'a, T> {
    GraphRef { data: self.data, borrow_count: self.borrow_count }
  }

  pub fn awake_ref(&self) -> GraphRefAwake<'a, T> {
    acquire_for_read(self.borrow_count);
    GraphRefAwake {
      data: unsafe { &*self.data },
      borrow_count: self.borrow_count,
    }
  }

  pub fn awake_mut(&self) -> GraphRefAwakeMut<'a, T> {
    acquire_for_write(self.borrow_count);
    GraphRefAwakeMut {
      data: unsafe { &mut *self.data },
      borrow_count: self.borrow_count,
    }
  }

  pub fn map<F, U>(&self, map_fn: F) -> GraphRefMut<'a, U>
  where
    F: FnOnce(&'a mut T) -> &'a mut U,
    U: 'a,
  {
    acquire_for_write(self.borrow_count);
    let ref_data: &'a mut T = unsafe { &mut *self.data };
    let new_ref_data = map_fn(ref_data);
    let new_data = new_ref_data as *mut _;
    self.borrow_count.set(0);
    GraphRefMut { data: new_data, borrow_count: self.borrow_count }
  }
}

impl<'a, T, U> CoerceUnsized<GraphRefMut<'a, U>> for GraphRefMut<'a, T>
where
  T: Unsize<U> + ?Sized + 'a,
  U: ?Sized + 'a,
{}

// =====

pub struct GraphRefAwake<'a, T: ?Sized + 'a> {
  data: &'a T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRefAwake<'a, T> {
  pub fn asleep(awake: &GraphRefAwake<'a, T>) -> GraphRef<'a, T> {
    GraphRef {
      data: awake.data as *const _,
      borrow_count: awake.borrow_count
    }
  }

  pub fn clone(orig: &GraphRefAwake<'a, T>) -> Self {
    orig.borrow_count.set(orig.borrow_count.get() + 1);
    GraphRefAwake { data: orig.data, borrow_count: orig.borrow_count }
  }

  pub fn map<F, U>(orig: GraphRefAwake<'a, T>, map_fn: F) -> GraphRefAwake<'a, U>
  where F: FnOnce(&'a T) -> &'a U
  {
    GraphRefAwake { data: map_fn(orig.data), borrow_count: orig.borrow_count }
  }
}

impl<'a, T: ?Sized + 'a> Drop for GraphRefAwake<'a, T> {
  fn drop(&mut self) {
    self.borrow_count.set(self.borrow_count.get() - 1);
  }
}

impl<'a, T: ?Sized + 'a> Deref for GraphRefAwake<'a, T> {
  type Target = T;
  fn deref(&self) -> &T {
    self.data
  }
}

impl<'a, T: Debug + 'a> Debug for GraphRefAwake<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    <T as Debug>::fmt(self.data, f)
  }
}

impl<'a, T: Display + 'a> Display for GraphRefAwake<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    <T as Display>::fmt(self.data, f)
  }
}

impl<'a, T, U> CoerceUnsized<GraphRefAwake<'a, U>> for GraphRefAwake<'a, T>
where
  T: Unsize<U> + ?Sized + 'a,
  U: ?Sized + 'a,
{}

// =====

pub struct GraphRefAwakeMut<'a, T: ?Sized + 'a> {
  data: &'a mut T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRefAwakeMut<'a, T> {
  pub fn asleep_ref(awake: &mut GraphRefAwakeMut<'a, T>) -> GraphRef<'a, T> {
    GraphRef { data: awake.data, borrow_count: awake.borrow_count }
  }

  pub fn asleep_mut(awake: &mut GraphRefAwakeMut<'a, T>) -> GraphRefMut<'a, T> {
    GraphRefMut { data: awake.data, borrow_count: awake.borrow_count }
  }

  pub fn awake_ref(awake_mut: GraphRefAwakeMut<'a, T>) -> GraphRefAwake<'a, T> {
    let (data, borrow_count) = (awake_mut.data as *mut _, awake_mut.borrow_count);
    mem::forget(awake_mut);
    // We know this is ok because if we're passed a valid
    // AwakeMut, it is the only active ref, so we can just
    // set this to 1.
    borrow_count.set(1);
    GraphRefAwake { data: unsafe { &*data }, borrow_count }
  }

  pub fn map<F, U>(orig: GraphRefAwakeMut<'a, T>, map_fn: F) -> GraphRefAwakeMut<'a, U>
  where F: FnOnce(&'a mut T) -> &'a mut U
  {
    GraphRefAwakeMut { data: map_fn(orig.data), borrow_count: orig.borrow_count }
  }
}

impl<'a, T: ?Sized + 'a> Drop for GraphRefAwakeMut<'a, T> {
  fn drop(&mut self) {
    self.borrow_count.set(0);
  }
}

impl<'a, T: ?Sized + 'a> Deref for GraphRefAwakeMut<'a, T> {
  type Target = T;
  fn deref(&self) -> &T {
    self.data
  }
}

impl<'a, T: ?Sized + 'a> DerefMut for GraphRefAwakeMut<'a, T> {
  fn deref_mut(&mut self) -> &mut T {
    self.data
  }
}

impl<'a, T: Debug + ?Sized + 'a> Debug for GraphRefAwakeMut<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    <T as Debug>::fmt(&*self, f)
  }
}

impl<'a, T: Display + ?Sized + 'a> Display for GraphRefAwakeMut<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    <T as Display>::fmt(&*self, f)
  }
}

impl<'a, T, U> CoerceUnsized<GraphRefAwakeMut<'a, U>> for GraphRefAwakeMut<'a, T>
where
  T: Unsize<U> + ?Sized + 'a,
  U: ?Sized + 'a,
{}
