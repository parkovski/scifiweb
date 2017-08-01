use std::time::Duration;
use futures::future::Future;
use instance::Target;
use instance::messaging::{Mailbox, MessagingError, Message, MessageLimit, MessageThread};
use sf_util::IntoBox;

pub type MessagingFuture<'a, Item> = Box<Future<Item = Item, Error = MessagingError> + Send + 'a>;

pub trait MessagingAccessor<'a>
  : MailboxAccessor<'a> + MessageThreadAccessor<'a> + MessageAccessor<'a> {
}
impl<'a, A> MessagingAccessor<'a> for A
where
  A: MailboxAccessor<'a> + MessageThreadAccessor<'a> + MessageAccessor<'a>,
{
}

pub trait MailboxAccessor<'a>: Send + Sync {
  fn create_mailbox(
    &self,
    owner: Target,
    name: &str,
    message_limit: MessageLimit,
    thread_limit: u32,
  ) -> MessagingFuture<'a, Mailbox>;

  fn get_mailbox_for_owner(&self, owner: Target, name: &str) -> MessagingFuture<'a, Mailbox>;

  fn get_mailbox_by_id(&self, id: u64) -> MessagingFuture<'a, Mailbox>;

  fn get_all_mailboxes(&self, owner: Target) -> MessagingFuture<'a, Vec<Mailbox>>;

  fn delete_mailbox_for_owner(&self, owner: Target, name: &str) -> MessagingFuture<'a, ()>;

  fn delete_mailbox_by_id(&self, id: u64) -> MessagingFuture<'a, ()>;

  fn delete_all_mailboxes(&self, owner: Target) -> MessagingFuture<'a, ()>;
}

pub trait MessageThreadAccessor<'a>: Send + Sync {
  fn create_thread(&self, mailbox_id: u64, sender: Target) -> MessagingFuture<'a, MessageThread>;

  fn get_thread_by_id(&self, id: u64) -> MessagingFuture<'a, MessageThread> {
    self
      .get_threads_by_id(&[id], true)
      .and_then(move |mut vec| {
        vec
          .drain(..)
          .nth(0)
          .ok_or_else(|| MessagingError::not_found("id", id.to_string().as_str()))
      })
      .into_box()
  }

  fn get_threads_by_id(
    &self,
    ids: &[u64],
    missing_is_error: bool,
  ) -> MessagingFuture<'a, Vec<MessageThread>>;

  fn get_all_threads(&self, mailbox_id: u64) -> MessagingFuture<'a, Vec<MessageThread>>;

  fn get_threads_for_sender(
    &self,
    mailbox_id: u64,
    sender: Target,
  ) -> MessagingFuture<'a, Vec<MessageThread>>;

  fn delete_thread(&self, id: u64) -> MessagingFuture<'a, ()>;

  fn delete_threads(&self, ids: &[u64]) -> MessagingFuture<'a, ()>;

  fn delete_all_threads(&self, mailbox_id: u64) -> MessagingFuture<'a, ()>;
}

pub trait MessageAccessor<'a>: Send + Sync {
  fn create_message(
    &self,
    thread_id: u64,
    sender: Target,
    content: &str,
    title: Option<&str>,
    expire: Option<Duration>,
  ) -> MessagingFuture<'a, Message>;

  fn get_all_messages(&self, thread_id: u64) -> MessagingFuture<'a, Vec<Message>>;

  fn delete_message(&self, id: u64) -> MessagingFuture<'a, ()>;

  fn delete_all_messages(&self, thread_id: u64) -> MessagingFuture<'a, ()>;
}
