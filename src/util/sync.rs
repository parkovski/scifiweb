use std::sync::LockResult;
pub use super::rwlock::*;

/*pub struct WaitForLock<'a, T: 'a, L, U, G: 'a, I: 'a, E: 'a, RW: 'a>
where L: Fn(&'a T) -> &'a FutureLockable<'a, RW, Guard=G> + 'a,
      U: FnOnce(LockResult<G>) -> Result<I, E> + 'a,
{
  object: Arc<T>,
  get_try_lock_result: L,
  use_item: Option<U>,
  phantom: PhantomData<&'a (G, I, E)>
}

impl<'a, T: 'a, L, U, G, I, E, RW> WaitForLock<'a, T, L, U, G, I, E, RW>
where L: Fn(&'a T) -> &'a FutureLockable<'a, RW, Guard=G>,
      U: FnOnce(LockResult<G>) -> Result<I, E>,
{
  pub fn new(object: Arc<T>, get_try_lock_result: L, use_item: U) -> Self {
    WaitForLock { object, get_try_lock_result, use_item: Some(use_item), phantom: PhantomData }
  }
}

impl<'a, T: 'a, L, U, G, I, E, RW> Future for WaitForLock<'a, T, L, U, G, I, E, RW>
where L: for<'r> Fn(&'r T) -> &'r FutureLockable<'r, RW, Guard=G>,
      U: FnOnce(LockResult<G>) -> Result<I, E>,
{
  type Item = I;
  type Error = E;

  fn poll(&mut self) -> Poll<I, E> {
    let lockable = (self.get_try_lock_result)(&self.object);
    let try_lock_result = lockable.future_lock();
    let lock_result = match try_lock_result {
      Err(TryLockError::WouldBlock) => return Ok(Async::NotReady),
      Err(TryLockError::Poisoned(pe)) => Err(pe),
      Ok(guard) => Ok(guard),
    };

    (self.use_item.take().unwrap())(lock_result).map(|item| Async::Ready(item))
  }
}
*/
/// For cases where a panic doesn't affect
/// other threads' ability to function.
pub trait Unpoisoned<G> {
  fn unpoisoned(self) -> G;
}

impl<G> Unpoisoned<G> for LockResult<G> {
  fn unpoisoned(self) -> G {
    match self {
      Ok(guard) => guard,
      Err(poison_error) => {
        debug!("Called unpoison() on poisoned lock");
        poison_error.into_inner()
      }
    }
  }
}
