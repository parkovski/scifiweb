use std::time::Duration;

use futures::future::Future;

use instance::Target;
use instance::mailbox::{
  Mailbox, MessageLimit, MailboxError, MessageThread, Message
};

pub type MailboxFuture<'a, Item> = Box<Future<Item=Item, Error=MailboxError> + 'a>;

pub trait MailboxAccessor<'a> {
  fn create_mailbox(&mut self, owner: Target<'a>, name: &str, message_limit: MessageLimit, thread_limit: u32)
    -> MailboxFuture<'a, Mailbox<'a>>;
  
  fn get_mailbox_for_owner(&self, owner: Target<'a>, name: &str)
    -> MailboxFuture<'a, Mailbox<'a>>;

  fn get_mailbox_by_id(&self, id: u64) -> MailboxFuture<'a, Mailbox<'a>>;

  fn get_all_mailboxes(&self, owner: Target<'a>)
    -> MailboxFuture<'a, Vec<Mailbox<'a>>>;

  fn delete_mailbox_for_owner(&mut self, owner: Target<'a>, name: &str)
    -> MailboxFuture<'a, ()>;

  fn delete_mailbox_by_id(&mut self, id: u64) -> MailboxFuture<'a, ()>;

  fn delete_all_mailboxes(&mut self, owner: Target<'a>) -> MailboxFuture<'a, ()>;
}

pub trait MessageThreadAccessor<'a> {
  fn create_thread(&mut self, mailbox_id: u64, sender: Target<'a>)
    -> MailboxFuture<'a, MessageThread<'a>>;

  fn get_thread_by_id(&self, id: u64) -> MailboxFuture<'a, MessageThread<'a>> {
    Box::new(self.get_threads_by_id(&[id], true).and_then(
      move |mut vec| vec.drain(..).take(1).next().ok_or_else(|| MailboxError::not_found("id", id.to_string().as_str()))
    ))
  }

  fn get_threads_by_id(&self, ids: &[u64], missing_is_error: bool) -> MailboxFuture<'a, Vec<MessageThread<'a>>>;

  fn get_all_threads(&self, mailbox_id: u64) -> MailboxFuture<'a, Vec<MessageThread<'a>>>;

  fn get_threads_for_sender(&self, mailbox_id: u64, sender: Target<'a>)
    -> MailboxFuture<'a, Vec<MessageThread<'a>>>;

  fn delete_thread(&mut self, id: u64) -> MailboxFuture<'a, ()>;

  fn delete_all_threads(&mut self, mailbox_id: u64) -> MailboxFuture<'a, ()>;
}

pub trait MessageAccessor<'a> {
  fn create_message(
    &mut self,
    thread_id: u64,
    sender: Target<'a>,
    content: &str,
    title: Option<&str>,
    expire: Option<Duration>,
  ) -> MailboxFuture<'a, Message<'a>>;

  fn get_all_messages(&self, thread_id: u64) -> MailboxFuture<'a, Vec<Message<'a>>>;

  fn delete_message(&mut self, id: u64) -> MailboxFuture<'a, ()>;

  fn delete_all_messages(&mut self, thread_id: u64) -> MailboxFuture<'a, ()>;
}