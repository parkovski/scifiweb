#![feature(try_trait)]
#![feature(coerce_unsized)]
#![feature(unsize)]

extern crate crossbeam;
extern crate futures;
#[macro_use]
extern crate log;
extern crate termcolor;
extern crate fxhash;

pub mod future;
pub mod graph_cell;
pub mod logger;
pub mod split_vec;
pub mod sync;

use std::sync::Arc;
use std::cell::RefCell;
use std::default::Default;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::{Hash, BuildHasher};
use fxhash::FxHashSet;
use futures::Future;
use future::SFFuture;

pub fn id<T>(t: T) -> T {
  t
}

/// With futures we can end up with big expression
/// chains - rather than wrap them in lots of parens,
/// just write `expr.pipe(some_fn)`.
pub trait Pipe<T, F: FnOnce(Self) -> T>: Sized {
  fn pipe(self, f: F) -> T;
}

impl<S, T, F: FnOnce(Self) -> T> Pipe<T, F> for S {
  fn pipe(self, f: F) -> T {
    f(self)
  }
}

/// To avoid double-boxing. When boxing
/// a struct s as an instance of a trait,
/// `s.into()` doesn't infer that the type
/// should be `Box<Trait>`. `Box::new`/`from(s)`
/// works but with futures you end up having
/// to wrap big expression chains in extra
/// parenthesis.
pub trait IntoBox<'a, T: 'a + ?Sized>: Sized + 'a {
  fn into_box(self) -> Box<T>;
}

impl<'a, T: 'a> IntoBox<'a, T> for Box<T> {
  fn into_box(self) -> Box<T> {
    self
  }
}

impl<'a, T: 'a, E: 'a> IntoBox<'a, Future<Item = T, Error = E> + 'a> for SFFuture<'a, T, E> {
  fn into_box(self) -> Box<Future<Item = T, Error = E> + 'a> {
    self.into_inner()
  }
}

impl<'a, T: 'a> IntoBox<'a, T> for T {
  fn into_box(self) -> Box<T> {
    Box::new(self)
  }
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Hash, Clone)]
struct SharedStringWrapper(Arc<str>);

impl Borrow<str> for SharedStringWrapper {
  fn borrow(&self) -> &str {
    &self.0
  }
}

pub struct SharedString {
  strings: RefCell<FxHashSet<SharedStringWrapper>>,
}

impl SharedString {
  pub fn new() -> Self {
    SharedString { strings: RefCell::new(Default::default()) }
  }

  pub fn get(&self, s: &str) -> Arc<str> {
    let mut strings = self.strings.borrow_mut();
    if let Some(ss) = strings.get(s) {
      return ss.0.clone();
    }
    let ss: Arc<str> = s.into();
    strings.insert(SharedStringWrapper(ss.clone()));
    ss
  }
}

pub trait InsertUnique<K, V> {
  fn insert_unique(&mut self, key: K, value: V) -> Result<(), (K, V)>;
}

impl<K: Hash + Eq, V, H: BuildHasher> InsertUnique<K, V> for HashMap<K, V, H> {
  fn insert_unique(&mut self, key: K, value: V) -> Result<(), (K, V)> {
    if self.contains_key(&key) {
      Err((key, value))
    } else {
      self.insert(key, value);
      Ok(())
    }
  }
}
