use std::sync::Arc;
use std::ops::Deref;
use atomic::{Atomic, Ordering};
use futures::Future;
use model::instance::Target;
use model::instance::messaging::*;
use model::access::messaging::*;
use util::IntoBox;
use util::sync::{FutureRwLock, Unpoisoned};
use super::cache::*;

pub struct MemoryAccessorInner {
  pub mailbox_cache: FutureRwLock<MailboxCache>,
  pub next_mailbox_id: Atomic<u64>,

  pub message_thread_cache: FutureRwLock<MessageThreadCache>,
  pub next_message_thread_id: Atomic<u64>,

  pub message_cache: FutureRwLock<MessageCache>,
  pub next_message_id: Atomic<u64>,
}

#[derive(Clone)]
pub struct MemoryAccessor {
  inner: Arc<MemoryAccessorInner>,
}

impl Deref for MemoryAccessor {
  type Target = MemoryAccessorInner;
  fn deref(&self) -> &MemoryAccessorInner {
    &self.inner
  }
}

impl MemoryAccessor {
  pub fn new() -> Self {
    MemoryAccessor {
      inner: Arc::new(MemoryAccessorInner {
        mailbox_cache: FutureRwLock::new(MailboxCache::new()),
        next_mailbox_id: Atomic::new(0),
        message_thread_cache: FutureRwLock::new(MessageThreadCache::new()),
        next_message_thread_id: Atomic::new(0),
        message_cache: FutureRwLock::new(MessageCache::new()),
        next_message_id: Atomic::new(0),
      })
    }
  }

  fn next_mailbox_id(&self) -> u64 {
    self.next_mailbox_id.fetch_add(1, Ordering::AcqRel)
  }

  fn next_message_thread_id(&self) -> u64 {
    self.next_message_thread_id.fetch_add(1, Ordering::AcqRel)
  }

  fn next_message_id(&self) -> u64 {
    self.next_message_id.fetch_add(1, Ordering::AcqRel)
  }
}

impl<'a> MailboxAccessor<'a> for MemoryAccessor {
  fn create_mailbox(
    &self,
    owner: Target,
    name: &str,
    message_limit: MessageLimit,
    thread_limit: u32,
  ) -> MessagingFuture<'a, Mailbox> {
    let mailbox = Mailbox::new(
      self.next_mailbox_id(),
      owner,
      name.to_string(),
      message_limit,
      thread_limit,
    );

    self
      .mailbox_cache
      .write(move |result| result.unpoisoned().put_mailbox(mailbox))
      .into_box()
  }

  fn get_mailbox_for_owner(&self, owner: Target, name: &str) -> MessagingFuture<'a, Mailbox> {
    let name = name.to_owned();
    self
      .mailbox_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_mailbox_for_owner(owner.clone(), name.as_str())
          .ok_or_else(|| {
            MessagingError::not_found("(owner, name)", format!("({}, {})", owner, name))
          })
      })
      .into_box()
  }

  fn get_mailbox_by_id(&self, id: u64) -> MessagingFuture<'a, Mailbox> {
    self
      .mailbox_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_mailbox_by_id(id)
          .ok_or_else(|| MessagingError::not_found("id", id))
      })
      .into_box()
  }

  fn get_all_mailboxes(&self, owner: Target) -> MessagingFuture<'a, Vec<Mailbox>> {
    self
      .mailbox_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_all_mailboxes(owner.clone())
          .ok_or_else(|| MessagingError::not_found("owner", owner))
      })
      .into_box()
  }

  fn delete_mailbox_for_owner(&self, owner: Target, name: &str) -> MessagingFuture<'a, ()> {
    let name = name.to_owned();
    let this = self.clone();
    self
      .mailbox_cache
      .write(move |result| {
        result
          .unpoisoned()
          .delete_mailbox_for_owner(owner, name.as_str())
      })
      .and_then(move |ids| this.delete_threads(&ids))
      .into_box()
  }

  fn delete_mailbox_by_id(&self, id: u64) -> MessagingFuture<'a, ()> {
    let this = self.clone();
    self
      .mailbox_cache
      .write(move |result| result.unpoisoned().delete_mailbox_by_id(id))
      .and_then(move |ids| this.delete_threads(&ids))
      .into_box()
  }

  fn delete_all_mailboxes(&self, owner: Target) -> MessagingFuture<'a, ()> {
    let this = self.clone();
    self
      .mailbox_cache
      .write(move |result| {
        result.unpoisoned().delete_all_mailboxes(owner)
      })
      .and_then(move |ids| this.delete_threads(&ids))
      .into_box()
  }
}

impl<'a> MessageThreadAccessor<'a> for MemoryAccessor {
  fn create_thread(&self, mailbox_id: u64, sender: Target) -> MessagingFuture<'a, MessageThread> {
    let thread = MessageThread::new(self.next_message_thread_id(), sender, None);
    let this = self.clone();
    self
      .message_thread_cache
      .write(move |result| result.unpoisoned().put_thread(thread.clone()))
      .and_then(move |thread| {
        this.mailbox_cache.write(move |result| {
          match result.unpoisoned().get_mailbox_by_id_mut(mailbox_id) {
            Some(mailbox) => mailbox.thread_ids_mut().push(thread.id()),
            None => {
              debug!(
                "Thread {} still exists even though mailbox was not found",
                thread.id
              );
              return Err(MessagingError::not_found("mailbox id", mailbox_id));
            }
          }
          Ok(thread)
        })
      })
      .into_box()
  }

  fn get_threads_by_id(
    &self,
    ids: &[u64],
    missing_is_error: bool,
  ) -> MessagingFuture<'a, Vec<MessageThread>> {
    let ids = Vec::from(ids);
    self
      .message_thread_cache
      .read(move |result| {
        let threads = result.unpoisoned().get_threads_by_id(&ids);
        let (found, not_found) = threads
          .into_iter()
          .zip(ids.iter())
          .partition::<Vec<_>, _>(|&(ref t, _id)| t.is_some());
        if missing_is_error {
          let not_found_ids = not_found.into_iter().fold(String::new(), |mut acc, pair| {
            if !acc.is_empty() {
              acc += ", "
            }
            acc += pair.1.to_string().as_str();
            acc
          });
          if !not_found_ids.is_empty() {
            return Err(MessagingError::not_found("thread ids", not_found_ids));
          }
        }
        Ok(found.into_iter().map(|pair| pair.0.unwrap()).collect())
      })
      .into_box()
  }

  fn get_all_threads(&self, mailbox_id: u64) -> MessagingFuture<'a, Vec<MessageThread>> {
    let this = self.clone();
    self
      .mailbox_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_mailbox_by_id(mailbox_id)
          .ok_or_else(|| MessagingError::not_found("mailbox id", mailbox_id))
      })
      .and_then(move |mailbox| {
        this.message_thread_cache.write(move |result| {
          Ok(
            result
              .unpoisoned()
              .get_threads_by_id(mailbox.thread_ids())
              .into_iter()
              .filter_map(|t| t)
              .collect(),
          )
        })
      })
      .into_box()
  }

  fn get_threads_for_sender(
    &self,
    mailbox_id: u64,
    sender: Target,
  ) -> MessagingFuture<'a, Vec<MessageThread>> {
    let this = self.clone();
    self
      .mailbox_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_mailbox_by_id(mailbox_id)
          .ok_or_else(|| MessagingError::not_found("mailbox id", mailbox_id))
      })
      .and_then(move |mailbox| {
        this.message_thread_cache.read(move |result| {
          Ok(
            result
              .unpoisoned()
              .get_threads_by_id(mailbox.thread_ids())
              .into_iter()
              .filter_map(|thread| {
                match thread {
                  Some(ref t) if t.sender() == sender => {}
                  _ => return None,
                }
                thread
              })
              .collect(),
          )
        })
      })
      .into_box()
  }

  fn delete_thread(&self, id: u64) -> MessagingFuture<'a, ()> {
    let this = self.clone();
    self
      .message_thread_cache
      .write(move |result| Ok(result.unpoisoned().delete_threads(&[id])))
      .and_then(move |ids| {
        this
          .message_cache
          .write(move |result| Ok(result.unpoisoned().delete_messages(&ids)))
      })
      .into_box()
  }

  fn delete_threads(&self, ids: &[u64]) -> MessagingFuture<'a, ()> {
    let ids = Vec::from(ids);
    let this = self.clone();
    self
      .message_thread_cache
      .write(move |result| Ok(result.unpoisoned().delete_threads(&ids)))
      .and_then(move |ids| {
        this
          .message_cache
          .write(move |result| Ok(result.unpoisoned().delete_messages(&ids)))
      })
      .into_box()
  }

  fn delete_all_threads(&self, mailbox_id: u64) -> MessagingFuture<'a, ()> {
    let (this, this2) = (self.clone(), self.clone());
    self
      .mailbox_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_mailbox_by_id(mailbox_id)
          .ok_or_else(|| MessagingError::not_found("mailbox id", mailbox_id))
      })
      .and_then(move |mailbox| {
        this.message_thread_cache.write(move |result| {
          Ok(result.unpoisoned().delete_threads(mailbox.thread_ids()))
        })
      })
      .and_then(move |ids| {
        this2
          .message_cache
          .write(move |result| Ok(result.unpoisoned().delete_messages(&ids)))
      })
      .into_box()
  }
}

impl<'a> MessageAccessor<'a> for MemoryAccessor {
  fn create_message(
    &self,
    thread_id: u64,
    sender: Target,
    content: &str,
    title: Option<&str>,
    expire: Option<::std::time::Duration>,
  ) -> MessagingFuture<'a, Message> {
    let message = Message::new(
      self.next_message_id(),
      sender,
      content.to_string(),
      title.map(|t| t.to_string()),
      expire,
    );
    let this = self.clone();
    self
      .message_cache
      .write(move |result| result.unpoisoned().put_message(message))
      .and_then(move |message| {
        this.message_thread_cache.write(move |result| {
          match result.unpoisoned().get_thread_by_id_mut(thread_id) {
            Some(thread) => {
              thread.message_ids_mut().push(message.id());
              Ok(message)
            }
            None => Err(MessagingError::not_found("thread id", thread_id)),
          }
        })
      })
      .into_box()
  }

  fn get_all_messages(&self, thread_id: u64) -> MessagingFuture<'a, Vec<Message>> {
    let this = self.clone();
    self
      .message_thread_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_thread_by_id(thread_id)
          .ok_or_else(|| MessagingError::not_found("thread id", thread_id))
      })
      .and_then(move |thread| {
        this.message_cache.read(move |result| {
          Ok(
            result
              .unpoisoned()
              .get_messages_by_id(thread.message_ids())
              .into_iter()
              .filter_map(|m| m)
              .collect(),
          )
        })
      })
      .into_box()
  }

  fn delete_message(&self, id: u64) -> MessagingFuture<'a, ()> {
    self
      .message_cache
      .write(move |result| Ok(result.unpoisoned().delete_messages(&[id])))
      .into_box()
  }

  fn delete_all_messages(&self, thread_id: u64) -> MessagingFuture<'a, ()> {
    let this = self.clone();
    self
      .message_thread_cache
      .read(move |result| {
        result
          .unpoisoned()
          .get_thread_by_id(thread_id)
          .ok_or_else(|| MessagingError::not_found("thread id", thread_id))
      })
      .and_then(move |thread| {
        this.message_cache.write(move |result| {
          Ok(result.unpoisoned().delete_messages(thread.message_ids()))
        })
      })
      .into_box()
  }
}
