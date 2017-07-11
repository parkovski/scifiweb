use std::rc::Rc;
use std::cell::RefCell;

use futures::future;

use instance::Target;
use instance::mailbox::*;

use instance::access::defs::*;

use super::cache::*;

pub struct MemoryAccessor<'a> {
  mailbox_cache: Rc<RefCell<MailboxCache<'a>>>,
  next_mailbox_id: u64,

  message_thread_cache: Rc<RefCell<MessageThreadCache<'a>>>,
  next_message_thread_id: u64,

  message_cache: Rc<RefCell<MessageCache<'a>>>,
  next_message_id: u64,
}

impl<'a> MemoryAccessor<'a> {
  pub fn new() -> Self {
    MemoryAccessor {
      mailbox_cache: Rc::new(RefCell::new(MailboxCache::new())),
      next_mailbox_id: 0,
      message_thread_cache: Rc::new(RefCell::new(MessageThreadCache::new())),
      next_message_thread_id: 0,
      message_cache: Rc::new(RefCell::new(MessageCache::new())),
      next_message_id: 0,
    }
  }

  fn next_mailbox_id(&mut self) -> u64 {
    let id = self.next_mailbox_id;
    self.next_mailbox_id += 1;
    id
  }

  fn next_message_thread_id(&mut self) -> u64 {
    let id = self.next_message_thread_id;
    self.next_message_thread_id += 1;
    id
  }

  fn next_message_id(&mut self) -> u64 {
    let id = self.next_message_id;
    self.next_message_id += 1;
    id
  }
}

impl<'a> MailboxAccessor<'a> for MemoryAccessor<'a> {
  fn create_mailbox(
    &mut self,
    owner: Target<'a>,
    name: &str,
    message_limit: MessageLimit,
    thread_limit: u32
  ) -> MailboxFuture<'a, Mailbox<'a>>
  {
    let mailbox = Mailbox::new(
      self.next_mailbox_id(),
      owner,
      name.to_string(),
      message_limit,
      thread_limit,
    );
    self.mailbox_cache.borrow_mut().put_mailbox(mailbox.clone());
    Box::new(future::result(Ok(mailbox)))
  }

  fn get_mailbox_for_owner(&self, owner: Target<'a>, name: &str)
    -> MailboxFuture<'a, Mailbox<'a>>
  {
    Box::new(future::result(
      self.mailbox_cache.borrow()
        .get_mailbox_for_owner(owner, name)
        .ok_or_else(|| MailboxError::not_found(
          "(owner, name)",
          format!("({}, {})", owner, name).as_str()
        ))
    ))
  }

  fn get_mailbox_by_id(&self, id: u64) -> MailboxFuture<'a, Mailbox<'a>> {
    Box::new(future::result(
      self.mailbox_cache.borrow()
        .get_mailbox_by_id(id)
        .ok_or_else(|| MailboxError::not_found("id", id.to_string().as_str()))
    ))
  }

  fn get_all_mailboxes(&self, owner: Target<'a>) -> MailboxFuture<'a, Vec<Mailbox<'a>>> {
    Box::new(future::result(self.mailbox_cache.borrow().get_all_mailboxes(owner)
      .ok_or_else(|| MailboxError::not_found("owner", owner.to_string().as_str()))
    ))
  }

  fn delete_mailbox_for_owner(&mut self, owner: Target<'a>, name: &str)
    -> MailboxFuture<'a, ()>
  {
    Box::new(future::result(
      if self.mailbox_cache.borrow_mut().delete_mailbox_for_owner(
        owner,
        name,
        &mut self.message_thread_cache.borrow_mut(),
        &mut self.message_cache.borrow_mut()
      )
      {
        Ok(())
      } else {
        Err(MailboxError::not_found("(owner, name)", format!("({}, {})", owner, name).as_str()))
      }
    ))
  }

  fn delete_mailbox_by_id(&mut self, id: u64) -> MailboxFuture<'a, ()> {
    Box::new(future::result(
      if self.mailbox_cache.borrow_mut().delete_mailbox_by_id(
        id,
        &mut self.message_thread_cache.borrow_mut(),
        &mut self.message_cache.borrow_mut(),
      )
      {
        Ok(())
      } else {
        Err(MailboxError::not_found("id", id.to_string().as_str()))
      }
    ))
  }

  fn delete_all_mailboxes(&mut self, owner: Target<'a>) -> MailboxFuture<'a, ()> {
    Box::new(future::result(
      if self.mailbox_cache.borrow_mut().delete_all_mailboxes(
        owner,
        &mut self.message_thread_cache.borrow_mut(),
        &mut self.message_cache.borrow_mut()
      )
      {
        Ok(())
      } else {
        Err(MailboxError::not_found("owner", owner.to_string().as_str()))
      }
    ))
  }
}

impl<'a> MessageThreadAccessor<'a> for MemoryAccessor<'a> {
  fn create_thread(&mut self, mailbox_id: u64, sender: Target<'a>)
    -> MailboxFuture<'a, MessageThread<'a>>
  {
    let mut mailbox = match self.mailbox_cache.borrow().get_mailbox_by_id(mailbox_id) {
      Some(mailbox) => mailbox,
      None => return MailboxError::not_found("mailbox id", mailbox_id.to_string().as_str()).into_future(),
    };
    let thread = MessageThread::new(
      self.next_message_thread_id(),
      sender,
      None,
    );
    self.message_thread_cache.borrow_mut().put_thread(thread.clone());
    mailbox.thread_ids_mut().push(thread.id());
    Box::new(future::result(Ok(thread)))
  }

  fn get_threads_by_id(&self, ids: &[u64], missing_is_error: bool) -> MailboxFuture<'a, Vec<MessageThread<'a>>> {
    let threads = self.message_thread_cache.borrow().get_threads_by_id(ids);
    let (found, not_found) = threads
      .into_iter()
      .zip(ids.iter())
      .partition::<Vec<_>, _>(|&(ref t, _id)| t.is_some());
    let not_found_ids = not_found
      .into_iter()
      .fold(String::new(), |mut acc, pair| {
        if acc.len() > 0 {
          acc += ", "
        }
        acc += pair.1.to_string().as_str();
        acc
      });
    if missing_is_error && not_found_ids.len() > 0 {
      MailboxError::not_found("thread ids", not_found_ids.as_str()).into_future()
    } else {
      Box::new(future::result(Ok(found.into_iter().filter_map(|pair| pair.0.clone()).collect())))
    }
  }

  fn get_all_threads(&self, mailbox_id: u64) -> MailboxFuture<'a, Vec<MessageThread<'a>>> {
    let mailbox = match self.mailbox_cache.borrow().get_mailbox_by_id(mailbox_id) {
      Some(mailbox) => mailbox,
      None => return MailboxError::not_found("mailbox id", mailbox_id.to_string().as_str()).into_future(),
    };
    let threads = self.message_thread_cache.borrow()
      .get_threads_by_id(mailbox.thread_ids().as_ref())
      .into_iter()
      .filter_map(|t| t)
      .collect();
    Box::new(future::result(Ok(
      threads
    )))
  }

  fn get_threads_for_sender(&self, mailbox_id: u64, sender: Target<'a>)
    -> MailboxFuture<'a, Vec<MessageThread<'a>>>
  {
    let mailbox = match self.mailbox_cache.borrow().get_mailbox_by_id(mailbox_id) {
      Some(mailbox) => mailbox,
      None => return MailboxError::not_found("mailbox id", mailbox_id.to_string().as_str()).into_future(),
    };
    let threads = self.message_thread_cache.borrow().get_threads_by_id(mailbox.thread_ids().as_ref());
    Box::new(future::result(Ok(
      threads.into_iter().filter_map(|thread| match thread {
        Some(ref t) if t.sender() == sender => thread.clone(),
        _ => None,
      }).collect()
    )))
  }

  fn delete_thread(&mut self, id: u64) -> MailboxFuture<'a, ()> {
    Box::new(future::result(
      match self.message_thread_cache.borrow_mut().delete_threads(
        &[id], &mut self.message_cache.borrow_mut()
      )
      {
        None => Ok(()),
        Some(_) => Err(MailboxError::not_found("thread id", id.to_string().as_str()))
      }
    ))
  }

  fn delete_all_threads(&mut self, mailbox_id: u64) -> MailboxFuture<'a, ()> {
    let mailbox = match self.mailbox_cache.borrow().get_mailbox_by_id(mailbox_id) {
      Some(mailbox) => mailbox,
      None => return MailboxError::not_found("mailbox id", mailbox_id.to_string().as_str()).into_future(),
    };
    self.message_thread_cache.borrow_mut().delete_threads(
      mailbox.thread_ids().as_ref(),
      &mut self.message_cache.borrow_mut()
    );
    Box::new(future::result(Ok(())))
  }
}

impl<'a> MessageAccessor<'a> for MemoryAccessor<'a> {
  fn create_message(
    &mut self,
    thread_id: u64,
    sender: Target<'a>,
    content: &str,
    title: Option<&str>,
    expire: Option<::std::time::Duration>,
  ) -> MailboxFuture<'a, Message<'a>>
  {
    let mut thread = match self.message_thread_cache.borrow().get_thread_by_id(thread_id) {
      Some(thread) => thread,
      None => return MailboxError::not_found("thread id", thread_id.to_string().as_str()).into_future(),
    };
    let message = Message::new(
      self.next_message_id(),
      sender,
      content.to_string(),
      title.map(|t| t.to_string()),
      expire,
    );
    self.message_cache.borrow_mut().put_message(message.clone());
    thread.message_ids_mut().push(message.id());
    Box::new(future::result(Ok(message)))
  }

  fn get_all_messages(&self, thread_id: u64) -> MailboxFuture<'a, Vec<Message<'a>>> {
    let thread = match self.message_thread_cache.borrow().get_thread_by_id(thread_id) {
      Some(thread) => thread,
      None => return MailboxError::not_found("thread id", thread_id.to_string().as_str()).into_future(),
    };
    let messages = self.message_cache.borrow()
      .get_messages_by_id(&thread.message_ids())
      .into_iter()
      .filter_map(|m| m)
      .collect();
    Box::new(future::result(Ok(messages)))
  }

  fn delete_message(&mut self, id: u64) -> MailboxFuture<'a, ()> {
    Box::new(future::result(
      match self.message_cache.borrow_mut().delete_messages(&[id]) {
        None => Ok(()),
        Some(_) => Err(MailboxError::not_found("message id", id.to_string().as_str())),
      }
    ))
  }

  fn delete_all_messages(&mut self, thread_id: u64) -> MailboxFuture<'a, ()> {
    let thread = match self.message_thread_cache.borrow().get_thread_by_id(thread_id) {
      Some(thread) => thread,
      None => return MailboxError::not_found("thread id", thread_id.to_string().as_str()).into_future(),
    };
    self.message_cache.borrow_mut().delete_messages(thread.message_ids().as_ref());
    Box::new(future::result(Ok(())))
  }
}