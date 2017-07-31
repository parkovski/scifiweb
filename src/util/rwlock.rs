use std::sync::{Arc, LockResult, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};
use crossbeam::sync::MsQueue;
use futures::{Async, Future, Poll};
use futures::task::{current, Task};

struct Data<T> {
  pub lock: RwLock<T>,
  pub write_waiters: MsQueue<Task>,
  pub read_waiters: MsQueue<Task>,
}

#[derive(Clone)]
pub struct FutureRwLock<T> {
  data: Arc<Data<T>>,
}

impl<T> FutureRwLock<T> {
  pub fn new(item: T) -> Self {
    FutureRwLock {
      data: Arc::new(Data {
        lock: RwLock::new(item),
        write_waiters: MsQueue::new(),
        read_waiters: MsQueue::new(),
      }),
    }
  }

  pub fn read<F, I, E>(&self, read_fn: F) -> ReadFuture<T, F, I, E>
  where
    F: for<'r> FnOnce(LockResult<RwLockReadGuard<'r, T>>) -> Result<I, E>,
  {
    ReadFuture {
      data: self.data.clone(),
      read_fn: Some(read_fn),
    }
  }

  pub fn write<F, I, E>(&self, write_fn: F) -> WriteFuture<T, F, I, E>
  where
    F: for<'r> FnOnce(LockResult<RwLockWriteGuard<'r, T>>) -> Result<I, E>,
  {
    WriteFuture {
      data: self.data.clone(),
      write_fn: Some(write_fn),
    }
  }
}

pub struct ReadFuture<T, F, I, E>
where
  F: for<'r> FnOnce(LockResult<RwLockReadGuard<'r, T>>) -> Result<I, E>,
{
  data: Arc<Data<T>>,
  read_fn: Option<F>,
}

impl<T, F, I, E> Future for ReadFuture<T, F, I, E>
where
  F: for<'r> FnOnce(LockResult<RwLockReadGuard<'r, T>>) -> Result<I, E>,
{
  type Item = I;
  type Error = E;

  fn poll(&mut self) -> Poll<I, E> {
    let lock_result = match self.data.lock.try_read() {
      Err(TryLockError::WouldBlock) => {
        self.data.read_waiters.push(current());
        return Ok(Async::NotReady);
      }
      Err(TryLockError::Poisoned(pe)) => Err(pe),
      Ok(guard) => Ok(guard),
    };

    let result = self.read_fn.take().unwrap()(lock_result).map(Async::Ready);
    if let Some(task) = self.data.write_waiters.try_pop() {
      task.notify();
    } else if let Some(task) = self.data.read_waiters.try_pop() {
      task.notify();
      while let Some(task) = self.data.read_waiters.try_pop() {
        task.notify();
      }
    }
    result
  }
}

pub struct WriteFuture<T, F, I, E>
where
  F: for<'r> FnOnce(LockResult<RwLockWriteGuard<'r, T>>) -> Result<I, E>,
{
  data: Arc<Data<T>>,
  write_fn: Option<F>,
}

impl<T, F, I, E> Future for WriteFuture<T, F, I, E>
where
  F: for<'r> FnOnce(LockResult<RwLockWriteGuard<'r, T>>) -> Result<I, E>,
{
  type Item = I;
  type Error = E;

  fn poll(&mut self) -> Poll<I, E> {
    let lock_result = match self.data.lock.try_write() {
      Err(TryLockError::WouldBlock) => {
        self.data.write_waiters.push(current());
        return Ok(Async::NotReady);
      }
      Err(TryLockError::Poisoned(pe)) => Err(pe),
      Ok(guard) => Ok(guard),
    };

    let result = self.write_fn.take().unwrap()(lock_result).map(Async::Ready);
    if let Some(task) = self.data.read_waiters.try_pop() {
      task.notify();
      while let Some(task) = self.data.read_waiters.try_pop() {
        task.notify();
      }
    } else if let Some(task) = self.data.write_waiters.try_pop() {
      task.notify();
    }
    result
  }
}
