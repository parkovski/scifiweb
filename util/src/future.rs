use std::ops::Try;
use futures::{Future, IntoFuture, Poll};
use super::IntoBox;

pub struct SFFuture<'a, Item, Error> {
  inner: Box<Future<Item = Item, Error = Error> + Send + 'a>,
}

impl<'a, Item, Error> SFFuture<'a, Item, Error> {
  pub fn new<F: IntoFuture<Item = Item, Error = Error> + 'a>(f: F) -> Self
  where
    F::Future: Send,
  {
    SFFuture {
      inner: f.into_future().into_box(),
    }
  }

  pub fn ok(item: Item) -> Self where Item: Send + 'a, Error: Send + 'a {
    Self::new(Ok(item))
  }

  pub fn err(error: Error) -> Self where Item: Send + 'a, Error: Send + 'a {
    Self::new(Err(error))
  }

  pub fn into_inner(self) -> Box<Future<Item = Item, Error = Error> + 'a> {
    self.inner
  }
}

impl<'a, Item, Error> Future for SFFuture<'a, Item, Error> {
  type Item = Item;
  type Error = Error;

  fn poll(&mut self) -> Poll<Item, Error> {
    self.inner.poll()
  }
}

impl<'a, Item: Send + 'a, Error: Send + 'a> Try for SFFuture<'a, Item, Error> {
  type Ok = Item;
  type Error = Error;

  fn into_result(self) -> Result<Item, Error> {
    warn!("Waiting on future via into_result (probably via Try/?)");
    self.wait()
  }

  fn from_error(v: Error) -> Self {
    SFFuture {
      inner: Box::new(Err(v).into_future()),
    }
  }

  fn from_ok(v: Item) -> Self {
    SFFuture {
      inner: Box::new(Ok(v).into_future()),
    }
  }
}
