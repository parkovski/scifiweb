use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;
use futures::{future, Future};
use model::instance::Target;
use model::instance::messaging::*;
use model::access::*;
use super::cache::*;

pub enum CacheExpireMode {
  Count(u32),
  Time(Duration),
}

pub struct CacheAccessor<'a> {
  expire_mode: CacheExpireMode,
  mailbox_cache: Rc<RefCell<MailboxCache<'a>>>,
  message_thread_cache: Rc<RefCell<MessageThreadCache<'a>>>,
  message_cache: Rc<RefCell<MessageCache<'a>>>,

  next: Rc<RefCell<&'a mut Accessor<'a>>>,
}

impl<'a> CacheAccessor<'a> {
  pub fn new(next: &'a mut Accessor<'a>, expire_mode: CacheExpireMode) -> Self {
    CacheAccessor {
      expire_mode,
      mailbox_cache: Rc::new(RefCell::new(MailboxCache::new())),
      message_thread_cache: Rc::new(RefCell::new(MessageThreadCache::new())),
      message_cache: Rc::new(RefCell::new(MessageCache::new())),
      next: Rc::new(RefCell::new(next)),
    }
  }

  fn get_or_cache_threads_with_filter<F>(
    next_accessor: Rc<RefCell<&'a mut Accessor<'a>>>,
    thread_cache: Rc<RefCell<MessageThreadCache<'a>>>,
    thread_ids: &[u64],
    missing_is_error: bool,
    filter: F,
  ) -> MessagingFuture<'a, Vec<MessageThread<'a>>>
    where F: for<'b> Fn(&'b MessageThread<'a>) -> bool + 'a
  {
    let cached_threads = thread_cache.borrow().get_threads_by_id(thread_ids);
    if { let has_empty = cached_threads.iter().any(|t| t.is_none()); has_empty } {
      let thread_cache = thread_cache.clone();
      let missing_threads = cached_threads.iter()
        .zip(thread_ids.into_iter())
        .filter_map(|(opt_thread, id)| if opt_thread.is_none() { Some(*id) } else { None })
        .collect::<Vec<_>>();
      Box::new(
        next_accessor.borrow()
          .get_threads_by_id(&missing_threads, missing_is_error)
          .then(|result| result.and_then(move |mut threads| {
            let mut thread_cache = thread_cache.borrow_mut();
            for thread in threads.iter() {
              thread_cache.put_thread(thread.clone());
            }
            threads.extend(cached_threads.into_iter().filter_map(move |opt_thread|
              opt_thread.and_then(|thread| if filter(&thread) { Some(thread) } else { None })
            ));
            Ok(threads)
          }))
      )
    } else {
      Box::new(future::result(Ok(
        cached_threads.into_iter().map(|t| t.unwrap()).filter(filter).collect()
      )))
    }
  }

  fn get_or_cache_threads(
    next_accessor: Rc<RefCell<&'a mut Accessor<'a>>>,
    thread_cache: Rc<RefCell<MessageThreadCache<'a>>>,
    thread_ids: &[u64],
    missing_is_error: bool
  ) -> MessagingFuture<'a, Vec<MessageThread<'a>>>
  {
    Self::get_or_cache_threads_with_filter(next_accessor, thread_cache, thread_ids, missing_is_error, |_| true)
  }
}

impl<'a> MailboxAccessor<'a> for CacheAccessor<'a> {
  fn create_mailbox(
    &mut self,
    owner: Target<'a>,
    name: &str,
    message_limit: MessageLimit,
    thread_limit: u32
  ) -> MessagingFuture<'a, Mailbox<'a>>
  {
    let mailbox_cache = self.mailbox_cache.clone();
    let future = self.next.borrow_mut()
      .create_mailbox(owner, name, message_limit, thread_limit)
      .then(|result| result.and_then(move |mailbox| {
        mailbox_cache.borrow_mut().put_mailbox(mailbox.clone());
        Ok(mailbox)
      }));
    Box::new(future)
  }

  fn get_mailbox_for_owner(&self, owner: Target<'a>, name: &str)
    -> MessagingFuture<'a, Mailbox<'a>>
  {
    if let Some(mailbox) = self.mailbox_cache.borrow().get_mailbox_for_owner(owner, name) {
      return Box::new(future::result(Ok(mailbox)));
    }
    let mailbox_cache = self.mailbox_cache.clone();
    let future = self.next.borrow()
      .get_mailbox_for_owner(owner, name)
      .then(move |result| {
        if let Ok(ref mailbox) = result {
          mailbox_cache.borrow_mut().put_mailbox(mailbox.clone());
        }
        result
      });
    Box::new(future)
  }

  fn get_mailbox_by_id(&self, id: u64) -> MessagingFuture<'a, Mailbox<'a>> {
    if let Some(mailbox) = self.mailbox_cache.borrow().get_mailbox_by_id(id) {
      return Box::new(future::result(Ok(mailbox)));
    }
    let mailbox_cache = self.mailbox_cache.clone();
    let future = self.next.borrow()
      .get_mailbox_by_id(id)
      .then(move |result| {
        if let Ok(ref mailbox) = result {
          mailbox_cache.borrow_mut().put_mailbox(mailbox.clone());
        }
        result
      });
    Box::new(future)
  }

  fn get_all_mailboxes(&self, owner: Target<'a>) -> MessagingFuture<'a, Vec<Mailbox<'a>>> {
    // Don't load this from cache - we don't know how many 'all' is.
    self.next.borrow().get_all_mailboxes(owner)
  }

  fn delete_mailbox_for_owner(
    &mut self,
    owner: Target<'a>,
    name: &str,
  ) -> MessagingFuture<'a, ()>
  {
    self.mailbox_cache.borrow_mut().delete_mailbox_for_owner(
      owner,
      name,
      &mut self.message_thread_cache.borrow_mut(),
      &mut self.message_cache.borrow_mut(),
    );
    self.next.borrow_mut().delete_mailbox_for_owner(owner, name)
  }

  fn delete_mailbox_by_id(&mut self, id: u64) -> MessagingFuture<'a, ()> {
    self.mailbox_cache.borrow_mut().delete_mailbox_by_id(
      id,
      &mut self.message_thread_cache.borrow_mut(),
      &mut self.message_cache.borrow_mut(),
    );
    self.next.borrow_mut().delete_mailbox_by_id(id)
  }

  fn delete_all_mailboxes(&mut self, owner: Target<'a>) -> MessagingFuture<'a, ()> {
    self.mailbox_cache.borrow_mut().delete_all_mailboxes(
      owner,
      &mut self.message_thread_cache.borrow_mut(),
      &mut self.message_cache.borrow_mut(),
    );
    self.next.borrow_mut().delete_all_mailboxes(owner)
  }
}

impl<'a> MessageThreadAccessor<'a> for CacheAccessor<'a> {
  fn create_thread(&mut self, mailbox_id: u64, sender: Target<'a>)
    -> MessagingFuture<'a, MessageThread<'a>>
  {
    let mailbox_cache = self.mailbox_cache.clone();
    let thread_cache = self.message_thread_cache.clone();
    let future = self.next.borrow_mut()
      .create_thread(mailbox_id, sender)
      .then(move |result| result.and_then(|thread| {
        thread_cache.borrow_mut().put_thread(thread.clone());
        if let Some(ref mut mailbox) = mailbox_cache.borrow().get_mailbox_by_id(mailbox_id) {
          mailbox.thread_ids_mut().push(thread.id());
        }
        Ok(thread)
      }));
    Box::new(future)
  }

  fn get_threads_by_id(&self, ids: &[u64], missing_is_error: bool) -> MessagingFuture<'a, Vec<MessageThread<'a>>> {
    Self::get_or_cache_threads(self.next.clone(), self.message_thread_cache.clone(), ids, missing_is_error)
  }

  fn get_all_threads(&self, mailbox_id: u64) -> MessagingFuture<'a, Vec<MessageThread<'a>>> {
    if let Some(mailbox) = self.mailbox_cache.borrow().get_mailbox_by_id(mailbox_id) {
      self.get_threads_by_id(&mailbox.thread_ids(), false)
    } else {
      let next_accessor = self.next.clone();
      let thread_cache = self.message_thread_cache.clone();
      let mailbox_cache = self.mailbox_cache.clone();
      Box::new(
        self.next.borrow().get_mailbox_by_id(mailbox_id)
          .and_then(move |mailbox| {
            let thread_ids = mailbox.thread_ids().clone();
            mailbox_cache.borrow_mut().put_mailbox(mailbox);
            Self::get_or_cache_threads(
              next_accessor,
              thread_cache,
              &thread_ids,
              false
            )
          })
      )
    }
  }

  fn get_threads_for_sender(&self, mailbox_id: u64, sender: Target<'a>)
    -> MessagingFuture<'a, Vec<MessageThread<'a>>>
  {
    if let Some(mailbox) = self.mailbox_cache.borrow().get_mailbox_by_id(mailbox_id) {
      Self::get_or_cache_threads_with_filter(
        self.next.clone(),
        self.message_thread_cache.clone(),
        &mailbox.thread_ids(),
        false,
        move |thread| thread.sender() == sender,
      )
    } else {
      self.next.borrow().get_threads_for_sender(mailbox_id, sender)
    }
  }

  fn delete_thread(&mut self, id: u64) -> MessagingFuture<'a, ()> {
    self.message_thread_cache.borrow_mut().delete_threads(&[id], &mut self.message_cache.borrow_mut());
    self.next.borrow_mut().delete_thread(id)
  }

  fn delete_all_threads(&mut self, mailbox_id: u64) -> MessagingFuture<'a, ()> {
    let mailbox_cache = self.mailbox_cache.borrow_mut();
    if let Some(mailbox) = mailbox_cache.get_mailbox_by_id(mailbox_id) {
      self.message_thread_cache.borrow_mut().delete_threads(&mailbox.thread_ids(), &mut self.message_cache.borrow_mut());
      self.next.borrow_mut().delete_all_threads(mailbox_id)
    } else {
      let thread_cache = self.message_thread_cache.clone();
      let message_cache = self.message_cache.clone();
      let next = self.next.clone();
      Box::new(self.next.borrow().get_mailbox_by_id(mailbox_id)
        .and_then(move |mailbox| {
          let thread_ids = &mailbox.thread_ids();
          let mut thread_cache = thread_cache.borrow_mut();
          let mut message_cache = message_cache.borrow_mut();
          let mut next = next.borrow_mut();
          thread_cache.delete_threads(thread_ids, &mut message_cache);
          next.delete_all_threads(mailbox_id)
        }))
    }
  }
}