use std::sync::{
  Arc, RwLock, RwLockReadGuard, RwLockWriteGuard,
  Mutex, MutexGuard, LockResult, TryLockError,
};
use futures::{Future, Poll, Async};

macro_rules! make_wait {
  ($name:ident, $type:ident, $guard:ident, $func:ident) => {
    pub struct $name<T, F, I, E>
    where F: FnOnce(LockResult<$guard<T>>) -> Result<I, E>
    {
      lock: Arc<$type<T>>,
      f: Option<F>,
    }

    impl<T, F, I, E> $name<T, F, I, E>
    where F: FnOnce(LockResult<$guard<T>>) -> Result<I, E>
    {
      pub fn new(lock: Arc<$type<T>>, f: F) -> Self {
        $name { lock, f: Some(f) }
      }
    }

    impl<T, F, I, E> Future for $name<T, F, I, E>
    where F: FnOnce(LockResult<$guard<T>>) -> Result<I, E>,
    {
      type Item = I;
      type Error = E;

      fn poll(&mut self) -> Poll<I, E> {
        let lock = self.lock.clone();
        let result = lock.$func();
        let lock_result: LockResult<_> = match result {
          Err(TryLockError::WouldBlock) => return Ok(Async::NotReady),
          Err(TryLockError::Poisoned(pe)) => Err(pe),
          Ok(guard) => Ok(guard),
        };

        match (self.f.take().unwrap())(lock_result) {
          Ok(item) => Ok(Async::Ready(item)),
          Err(error) => Err(error),
        }
      }
    }
  }
}

make_wait!(WaitReadRwLock, RwLock, RwLockReadGuard, try_read);
make_wait!(WaitWriteRwLock, RwLock, RwLockWriteGuard, try_write);
make_wait!(WaitMutex, Mutex, MutexGuard, try_lock);

/// For cases where a panic doesn't affect
/// other threads' ability to function.
pub trait LockAlways<G> {
  fn always(self) -> G;
}

impl<G> LockAlways<G> for LockResult<G> {
  fn always(self) -> G {
    match self {
      Ok(guard) => guard,
      Err(poison_error) => poison_error.into_inner(),
    }
  }
}