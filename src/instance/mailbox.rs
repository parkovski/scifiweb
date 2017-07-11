use std::time::Duration;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::cell::{ RefCell, Ref, RefMut };

use futures::{ future, Future };

use super::Target;

#[derive(Clone)]
pub struct Message<'a> {
  id: u64,
  sender: Target<'a>,
  content: Rc<RefCell<String>>,
  title: Option<Rc<RefCell<String>>>,
  expire: Option<Duration>,
}

impl<'a> Message<'a> {
  pub fn new(
    id: u64,
    sender: Target<'a>,
    content: String,
    title: Option<String>,
    expire: Option<Duration>,
  ) -> Self
  {
    Message {
      id,
      sender,
      content: Rc::new(RefCell::new(content)),
      title: title.map(|t| Rc::new(RefCell::new(t))),
      expire
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn expire(&self) -> Option<Duration> {
    self.expire
  }

  pub fn title<'b>(&'b self) -> Option<Ref<'b, String>> {
    match self.title {
      Some(ref title) => Some(title.borrow()),
      None => None,
    }
  }

  pub fn content<'b>(&'b self) -> Ref<'b, String> {
    self.content.borrow()
  }
}

#[derive(Clone)]
pub struct MessageThread<'a> {
  id: u64,
  sender: Target<'a>,
  latest_message: Option<Message<'a>>,
  message_ids: Rc<RefCell<Vec<u64>>>,
}

impl<'a> MessageThread<'a> {
  pub fn new(id: u64, sender: Target<'a>, latest_message: Option<Message<'a>>) -> Self {
    MessageThread {
      id,
      sender,
      latest_message,
      message_ids: Rc::new(RefCell::new(Vec::new())),
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn sender(&self) -> Target<'a> {
    self.sender
  }

  pub fn latest_message(&self) -> Option<&Message> {
    self.latest_message.as_ref()
  }

  pub fn message_ids<'b>(&'b self) -> Ref<'b, Vec<u64>> {
    self.message_ids.borrow()
  }

  pub(in instance) fn message_ids_mut<'b>(&'b mut self) -> RefMut<'b, Vec<u64>> {
    self.message_ids.borrow_mut()
  }
}

#[derive(Debug, Copy, Clone)]
pub enum MessageLimit {
  None,
  Duration(Duration),
  Count(u32),
}

#[derive(Clone)]
pub struct Mailbox<'a> {
  id: u64,
  owner: Target<'a>,
  name: String,
  message_limit: MessageLimit,
  thread_limit: u32,
  thread_ids: Rc<RefCell<Vec<u64>>>,
}

impl<'a> Mailbox<'a> {
  pub fn new(
    id: u64,
    owner: Target<'a>,
    name: String,
    message_limit: MessageLimit,
    thread_limit: u32,
  ) -> Self
  {
    Mailbox {
      id,
      owner,
      name,
      message_limit,
      thread_limit,
      thread_ids: Rc::new(RefCell::new(Vec::new())),
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn owner(&self) -> Target<'a> {
    self.owner
  }

  pub fn name(&self) -> &String {
    &self.name
  }

  pub fn message_limit(&self) -> MessageLimit {
    self.message_limit
  }

  pub fn thread_limit(&self) -> u32 {
    self.thread_limit
  }

  pub fn thread_ids<'b>(&'b self) -> Ref<'b, Vec<u64>> {
    self.thread_ids.borrow()
  }

  pub(in instance) fn thread_ids_mut<'b>(&'b mut self) -> RefMut<'b, Vec<u64>> {
    self.thread_ids.borrow_mut()
  }
}

#[derive(Debug)]
pub enum MailboxErrorKind {
  NoAccessor,
  NotFound,
  OperationNotSupported,
}

#[derive(Debug)]
pub struct MailboxError {
  kind: MailboxErrorKind,
  description: String,
}

impl MailboxError {
  pub fn no_accessor() -> Self {
    MailboxError {
      kind: MailboxErrorKind::NoAccessor,
      description: "No mailbox accessor".to_string(),
    }
  }

  pub fn not_found(index_type: &str, index: &str) -> Self {
    MailboxError {
      kind: MailboxErrorKind::NotFound,
      description: format!("Mailbox index {} (type {}) not found", index, index_type),
    }
  }

  pub fn operation_not_supported(operation: &str) -> Self {
    MailboxError {
      kind: MailboxErrorKind::OperationNotSupported,
      description: format!("Mailbox operation not supported: {}", operation),
    }
  }

  pub fn into_future<'a, T: 'a>(self) -> Box<Future<Item=T, Error=Self> + 'a> {
    Box::new(future::result(Err(self)))
  }
}

impl fmt::Display for MailboxError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", &self.description)
  }
}

impl Error for MailboxError {
  fn description(&self) -> &str {
    self.description.as_str()
  }
}