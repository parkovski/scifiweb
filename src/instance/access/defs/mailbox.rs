use std::time::Duration;
use futures::future::Future;
use instance::Target;
use instance::mailbox::{Mailbox, MailboxError, Message, MessageLimit, MessageThread};
use util::IntoBox;

pub type MailboxFuture<'a, Item> = Box<Future<Item = Item, Error = MailboxError> + Send + 'a>;

pub trait MessagingAccessor<'a>
  : MailboxAccessor<'a> + MessageThreadAccessor<'a> + MessageAccessor<'a> {
}
impl<'a, A> MessagingAccessor<'a> for A
where
  A: MailboxAccessor<'a> + MessageThreadAccessor<'a> + MessageAccessor<'a>,
{
}

pub trait MailboxAccessor<'a>: Clone + Send + Sync {
  fn create_mailbox(
    &self,
    owner: Target,
    name: &str,
    message_limit: MessageLimit,
    thread_limit: u32,
  ) -> MailboxFuture<'a, Mailbox>;

  fn get_mailbox_for_owner(&self, owner: Target, name: &str) -> MailboxFuture<'a, Mailbox>;

  fn get_mailbox_by_id(&self, id: u64) -> MailboxFuture<'a, Mailbox>;

  fn get_all_mailboxes(&self, owner: Target) -> MailboxFuture<'a, Vec<Mailbox>>;

  fn delete_mailbox_for_owner(&self, owner: Target, name: &str) -> MailboxFuture<'a, ()>;

  fn delete_mailbox_by_id(&self, id: u64) -> MailboxFuture<'a, ()>;

  fn delete_all_mailboxes(&self, owner: Target) -> MailboxFuture<'a, ()>;
}

pub trait MessageThreadAccessor<'a>: Clone + Send + Sync {
  fn create_thread(&self, mailbox_id: u64, sender: Target) -> MailboxFuture<'a, MessageThread>;

  fn get_thread_by_id(&self, id: u64) -> MailboxFuture<'a, MessageThread> {
    self
      .get_threads_by_id(&[id], true)
      .and_then(move |mut vec| {
        vec
          .drain(..)
          .nth(0)
          .ok_or_else(|| MailboxError::not_found("id", id.to_string().as_str()))
      })
      .into_box()
  }

  fn get_threads_by_id(
    &self,
    ids: &[u64],
    missing_is_error: bool,
  ) -> MailboxFuture<'a, Vec<MessageThread>>;

  fn get_all_threads(&self, mailbox_id: u64) -> MailboxFuture<'a, Vec<MessageThread>>;

  fn get_threads_for_sender(
    &self,
    mailbox_id: u64,
    sender: Target,
  ) -> MailboxFuture<'a, Vec<MessageThread>>;

  fn delete_thread(&self, id: u64) -> MailboxFuture<'a, ()>;

  fn delete_threads(&self, ids: &[u64]) -> MailboxFuture<'a, ()>;

  fn delete_all_threads(&self, mailbox_id: u64) -> MailboxFuture<'a, ()>;
}

pub trait MessageAccessor<'a>: Clone + Send + Sync {
  fn create_message(
    &self,
    thread_id: u64,
    sender: Target,
    content: &str,
    title: Option<&str>,
    expire: Option<Duration>,
  ) -> MailboxFuture<'a, Message>;

  fn get_all_messages(&self, thread_id: u64) -> MailboxFuture<'a, Vec<Message>>;

  fn delete_message(&self, id: u64) -> MailboxFuture<'a, ()>;

  fn delete_all_messages(&self, thread_id: u64) -> MailboxFuture<'a, ()>;
}
