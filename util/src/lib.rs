#![feature(try_trait)]

extern crate crossbeam;
extern crate futures;
#[macro_use]
extern crate log;
extern crate termcolor;

pub mod future;
pub mod logger;
pub mod sync;

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
pub trait IntoBox<'a, T: 'a>: Sized + 'a {
  fn into_box(self) -> Box<T>;
}

impl<'a, T: 'a> IntoBox<'a, T> for Box<T> {
  fn into_box(self) -> Box<T> {
    self
  }
}

impl<'a, T: 'a> IntoBox<'a, T> for T {
  fn into_box(self) -> Box<T> {
    Box::new(self)
  }
}
