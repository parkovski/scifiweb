use std::cell::{Cell, UnsafeCell};
use std::ops::{Deref, DerefMut, CoerceUnsized};
use std::marker::{Unsize, Copy};
use std::clone::Clone;
use std::hash::{Hash, Hasher};
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::mem;
use serde::{Serialize, Serializer};
use serde::ser::{SerializeStruct};

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
/*
  /// Returns a box because you can't move something that's
  /// got pointers to it. It looks like eventually Boxed
  /// would cover returning a different smart pointer,
  /// but they don't even implement it on nightly yet.
  pub fn self_referential<'a, N>(make_new: N) -> Box<GraphCell<T>>
  where
    T: 'a,
    N: FnOnce(GraphRef<'a, T>) -> T,
  {
    let cell = Box::new(GraphCell {
      borrow_count: Cell::new(0),
      data: UnsafeCell::new(unsafe { mem::uninitialized() }),
    });
    let self_ref = cell.asleep();
    // Take a reference so it will panic if you try
    // to use the uninitialized reference during initialization.
    let awake_ref = cell.awake_mut();
    // I think placement new would be ideal here, but this works too.
    mem::forget(mem::replace(unsafe { &mut *cell.data.get() }, make_new(self_ref)));
    cell
  }
*/
}

impl<T: ?Sized> GraphCell<T> {
  pub fn asleep<'a>(&self) -> GraphRef<'a, T> where Self: 'a {
    GraphRef {
      data: self.data.get(),
      borrow_count: unsafe { self.borrow_count() },
    }
  }

  pub fn asleep_mut<'a>(&self) -> GraphRefMut<'a, T> where Self: 'a {
    GraphRefMut {
      data: self.data.get(),
      borrow_count: unsafe { self.borrow_count() },
    }
  }

  pub fn awake<'a>(&'a self) -> GraphRefAwake<'a, T> where Self: 'a {
    acquire_for_read(&self.borrow_count);
    unsafe {
      GraphRefAwake {
        data: &*self.data.get(),
        borrow_count: self.borrow_count(),
      }
    }
  }

  pub fn awake_mut<'a>(&'a self) -> GraphRefAwakeMut<'a, T> where Self: 'a {
    acquire_for_write(&self.borrow_count);
    unsafe {
      GraphRefAwakeMut {
        data: &mut *self.data.get(),
        borrow_count: self.borrow_count(),
      }
    }
  }

  unsafe fn borrow_count<'a>(&self) -> &'a Cell<usize> where Self: 'a {
    &*(&self.borrow_count as *const _)
  }
}

impl<T: Debug + ?Sized> Debug for GraphCell<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.debug_struct("GraphCell")
      .field("data", &self.awake())
      .field("borrow_count", &self.borrow_count.get())
      .finish()
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

impl<T: Serialize> Serialize for GraphCell<T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_struct("GraphCell", 2)?;
    state.serialize_field("data", &*self.awake())?;
    state.serialize_field("ptr", &format!("{:p}", self.data.get()))?;
    state.end()
  }
}

// =====

#[derive(Debug)]
pub struct GraphRef<'a, T: ?Sized + 'a> {
  data: *const T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRef<'a, T> {
  pub fn awake<'b>(&'b self) -> GraphRefAwake<'b, T> where 'a: 'b {
    acquire_for_read(self.borrow_count);
    unsafe {
      GraphRefAwake {
        data: &*self.data,
        borrow_count: &*(self.borrow_count as *const _),
      }
    }
  }

  fn map_data<'b, F, U>(&self, map_fn: F) -> U
  where
    'a: 'b,
    F: FnOnce(&'b T) -> U + 'b,
    U: 'b,
  {
    acquire_for_read(self.borrow_count);
    let ref_data: &'b T = unsafe { &*self.data };
    let new_data = map_fn(ref_data);
    self.borrow_count.set(self.borrow_count.get() - 1);
    new_data
  }

  pub fn map<'b, F, U>(&'b self, map_fn: F) -> GraphRef<'a, U>
  where
    'a: 'b,
    F: FnOnce(&'b T) -> &'b U + 'b,
    U: ?Sized + 'a,
  {
    let new_data = self.map_data(map_fn) as *const _;
    GraphRef {
      data: new_data,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    }
  }

  pub fn map_opt<'b, F, U>(&'b self, map_fn: F) -> Option<GraphRef<'a, U>>
  where
    'a: 'b,
    F: (FnOnce(&'b T) -> Option<&'b U>) + 'b,
    U: ?Sized + 'a,
  {
    self.map_data(map_fn).map(|data| GraphRef {
      data: data as *const _,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    })
  }

  pub fn map_res<'b, F, U, E>(&'b self, map_fn: F) -> Result<GraphRef<'a, U>, E>
  where
    'a: 'b,
    F: FnOnce(&'b T) -> Result<&'b U, E> + 'b,
    U: ?Sized + 'a,
    E: 'b,
  {
    self.map_data(map_fn).map(|data| GraphRef {
      data: data as *const _,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    })
  }
}

impl<'a, T: ?Sized + 'a> Clone for GraphRef<'a, T> {
  fn clone(&self) -> Self {
    GraphRef {
      data: self.data,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    }
  }
}

impl<'a, T: ?Sized + 'a> Copy for GraphRef<'a, T> {}

impl<'a, T, U> CoerceUnsized<GraphRef<'a, U>> for GraphRef<'a, T>
where
  T: Unsize<U> + ?Sized + 'a,
  U: ?Sized + 'a,
{}

impl<'a, T: ?Sized + 'a> Serialize for GraphRef<'a, T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_newtype_struct(
      "GraphRef", &format!("{:p}", self.data)
    )
  }
}

// =====

#[derive(Debug)]
pub struct GraphRefMut<'a, T: ?Sized + 'a> {
  data: *mut T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRefMut<'a, T> {
  pub fn asleep_ref(&self) -> GraphRef<'a, T> {
    GraphRef {
      data: self.data,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    }
  }

  pub fn awake<'b>(&'b self) -> GraphRefAwake<'b, T> where 'a: 'b {
    acquire_for_read(self.borrow_count);
    unsafe {
      GraphRefAwake {
        data: &*self.data,
        borrow_count: &*(self.borrow_count as *const _),
      }
    }
  }

  pub fn awake_mut<'b>(&'b self) -> GraphRefAwakeMut<'b, T> where 'a: 'b {
    acquire_for_write(self.borrow_count);
    unsafe {
      GraphRefAwakeMut {
        data: &mut *self.data,
        borrow_count: &*(self.borrow_count as *const _),
      }
    }
  }

  fn map_data<'b, F, U>(&self, map_fn: F) -> U
  where
    'a: 'b,
    F: FnOnce(&'b mut T) -> U + 'b,
    U: 'b,
  {
    acquire_for_write(self.borrow_count);
    let ref_data: &'a mut T = unsafe { &mut *self.data };
    let new_data = map_fn(ref_data);
    self.borrow_count.set(0);
    new_data
  }

  pub fn map<'b, F, U>(&'b self, map_fn: F) -> GraphRefMut<'a, U>
  where
    'a: 'b,
    F: FnOnce(&'b mut T) -> &'b mut U + 'b,
    U: ?Sized + 'a,
  {
    let new_data = self.map_data(map_fn) as *mut _;
    GraphRefMut {
      data: new_data,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    }
  }

  pub fn map_opt<'b, F, U>(&'b self, map_fn: F) -> Option<GraphRefMut<'a, U>>
  where
    'a: 'b,
    F: (FnOnce(&'b mut T) -> Option<&'b mut U>) + 'b,
    U: ?Sized + 'a,
  {
    self.map_data(map_fn).map(|data| GraphRefMut {
      data: data as *mut _,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    })
  }

  pub fn map_res<'b, F, U, E>(&'b self, map_fn: F) -> Result<GraphRefMut<'a, U>, E>
  where
    'a: 'b,
    F: FnOnce(&'b mut T) -> Result<&'b mut U, E> + 'b,
    U: ?Sized + 'a,
    E: 'b,
  {
    self.map_data(map_fn).map(|data| GraphRefMut {
      data: data as *mut _,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    })
  }
}

impl<'a, T: ?Sized + 'a> Clone for GraphRefMut<'a, T> {
  fn clone(&self) -> Self {
    GraphRefMut {
      data: self.data,
      borrow_count: unsafe { &*(self.borrow_count as *const _) },
    }
  }
}

impl<'a, T: ?Sized + 'a> Copy for GraphRefMut<'a, T> {}

impl<'a, T, U> CoerceUnsized<GraphRefMut<'a, U>> for GraphRefMut<'a, T>
where
  T: Unsize<U> + ?Sized + 'a,
  U: ?Sized + 'a,
{}

impl<'a, T: ?Sized + 'a> Serialize for GraphRefMut<'a, T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_newtype_struct(
      "GraphRefMut",
      &format!("{:p}", self.data)
    )
  }
}

// =====

pub struct GraphRefAwake<'a, T: ?Sized + 'a> {
  data: &'a T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRefAwake<'a, T> {
  pub fn asleep(awake: &GraphRefAwake<'a, T>) -> GraphRef<'a, T> {
    GraphRef {
      data: awake.data as *const _,
      borrow_count: unsafe { &*(awake.borrow_count as *const _) },
    }
  }

  pub fn clone(orig: &GraphRefAwake<'a, T>) -> Self {
    orig.borrow_count.set(orig.borrow_count.get() + 1);
    GraphRefAwake {
      data: orig.data,
      borrow_count: unsafe { &*(orig.borrow_count as *const _) },
    }
  }

  pub fn map<F, U>(orig: GraphRefAwake<'a, T>, map_fn: F) -> GraphRefAwake<'a, U>
  where F: FnOnce(&'a T) -> &'a U
  {
    GraphRefAwake {
      data: map_fn(orig.data),
      borrow_count: orig.borrow_count
    }
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

impl<'a, T: Debug + ?Sized + 'a> Debug for GraphRefAwake<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    <T as Debug>::fmt(self.data, f)
  }
}

impl<'a, T: Display + ?Sized + 'a> Display for GraphRefAwake<'a, T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    <T as Display>::fmt(self.data, f)
  }
}

impl<'a, T, U> CoerceUnsized<GraphRefAwake<'a, U>> for GraphRefAwake<'a, T>
where
  T: Unsize<U> + ?Sized + 'a,
  U: ?Sized + 'a,
{}

impl<'a, T: ?Sized + Serialize + 'a> Serialize for GraphRefAwake<'a, T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_newtype_struct("GraphRefAwake", self.data)
  }
}

// =====

pub struct GraphRefAwakeMut<'a, T: ?Sized + 'a> {
  data: &'a mut T,
  borrow_count: &'a Cell<usize>,
}

impl<'a, T: ?Sized + 'a> GraphRefAwakeMut<'a, T> {
  pub fn asleep_ref(awake: &mut GraphRefAwakeMut<'a, T>) -> GraphRef<'a, T> {
    GraphRef {
      data: awake.data,
      borrow_count: unsafe { &*(awake.borrow_count as *const _) },
    }
  }

  pub fn asleep_mut(awake: &mut GraphRefAwakeMut<'a, T>) -> GraphRefMut<'a, T> {
    GraphRefMut {
      data: awake.data,
      borrow_count: unsafe { &*(awake.borrow_count as *const _) },
    }
  }

  pub fn awake(awake_mut: GraphRefAwakeMut<'a, T>) -> GraphRefAwake<'a, T> {
    let (data, borrow_count) = (awake_mut.data as *mut _, awake_mut.borrow_count);
    mem::forget(awake_mut);
    // We know this is ok because if we're passed a valid
    // AwakeMut, it is the only active ref, so we can just
    // set this to 1.
    borrow_count.set(1);
    unsafe {
      GraphRefAwake {
        data: &*data,
        borrow_count: &*(borrow_count as *const _),
      }
    }
  }

  pub fn map<F, U>(orig: GraphRefAwakeMut<'a, T>, map_fn: F) -> GraphRefAwakeMut<'a, U>
  where F: FnOnce(&'a mut T) -> &'a mut U
  {
    GraphRefAwakeMut {
      data: map_fn(orig.data),
      borrow_count: unsafe { &*(orig.borrow_count as *const _) },
    }
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

impl<'a, T: ?Sized + Serialize + 'a> Serialize for GraphRefAwakeMut<'a, T> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_newtype_struct("GraphRefAwakeMut", self.data)
  }
}
