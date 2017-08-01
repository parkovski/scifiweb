use std::time::Duration;
use std::error::Error;
use std::fmt::{self, Display};
use std::str::FromStr;
use futures::{future, Future};
use rules::error::FormatError;
use super::Target;

#[derive(Debug, Clone)]
pub struct Message {
  pub id: u64,
  pub sender: Target,
  pub content: Box<str>,
  pub title: Option<Box<str>>,
  pub expire: Option<Duration>,
}

impl Message {
  pub fn new(
    id: u64,
    sender: Target,
    content: String,
    title: Option<String>,
    expire: Option<Duration>,
  ) -> Self {
    Message {
      id,
      sender,
      content: content.into_boxed_str(),
      title: title.map(String::into_boxed_str),
      expire,
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn expire(&self) -> Option<Duration> {
    self.expire
  }

  pub fn title(&self) -> Option<&str> {
    self.title.as_ref().map(Box::as_ref)
  }

  pub fn content(&self) -> &str {
    &self.content
  }
}

#[derive(Debug, Clone)]
pub struct MessageThread {
  pub id: u64,
  pub sender: Target,
  pub latest_message: Option<Message>,
  pub message_ids: Vec<u64>,
}

impl MessageThread {
  pub fn new(id: u64, sender: Target, latest_message: Option<Message>) -> Self {
    MessageThread {
      id,
      sender,
      latest_message,
      message_ids: Vec::new(),
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn sender(&self) -> Target {
    self.sender.clone()
  }

  pub fn latest_message(&self) -> Option<&Message> {
    self.latest_message.as_ref()
  }

  pub fn message_ids(&self) -> &[u64] {
    self.message_ids.as_ref()
  }

  pub fn message_ids_mut(&mut self) -> &mut Vec<u64> {
    &mut self.message_ids
  }
}

#[derive(Debug, Copy, Clone)]
pub enum MessageLimit {
  None,
  Duration(Duration),
  Count(u32),
}

impl MessageLimit {
  fn format_error() -> FormatError {
    FormatError::new("none, count, or duration in seconds (#s)")
  }
}

impl FromStr for MessageLimit {
  type Err = FormatError;

  fn from_str(s: &str) -> Result<Self, FormatError> {
    let is_seconds = {
      let b = s.as_bytes();
      b[b.len() - 1] == b's'
    };

    if s == "none" {
      Ok(MessageLimit::None)
    } else if is_seconds {
      s[0..s.len() - 1]
        .parse::<u64>()
        .map(|secs| MessageLimit::Duration(Duration::new(secs, 0)))
        .map_err(|_| Self::format_error())
    } else {
      s.parse::<u32>()
        .map(MessageLimit::Count)
        .map_err(|_| Self::format_error())
    }
  }
}

#[derive(Debug, Clone)]
pub struct Mailbox {
  pub id: u64,
  pub owner: Target,
  pub name: String,
  pub message_limit: MessageLimit,
  pub thread_limit: u32,
  pub thread_ids: Vec<u64>,
}

impl Mailbox {
  pub fn new(
    id: u64,
    owner: Target,
    name: String,
    message_limit: MessageLimit,
    thread_limit: u32,
  ) -> Self {
    Mailbox {
      id,
      owner,
      name,
      message_limit,
      thread_limit,
      thread_ids: Vec::new(),
    }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn owner(&self) -> Target {
    self.owner.clone()
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn message_limit(&self) -> MessageLimit {
    self.message_limit
  }

  pub fn thread_limit(&self) -> u32 {
    self.thread_limit
  }

  pub fn thread_ids(&self) -> &[u64] {
    self.thread_ids.as_ref()
  }

  pub fn thread_ids_mut(&mut self) -> &mut Vec<u64> {
    &mut self.thread_ids
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MessagingErrorKind {
  NoAccessor,
  NotFound,
  OperationNotSupported,
  AlreadyExists,
}

#[derive(Debug, Clone)]
pub struct MessagingError {
  kind: MessagingErrorKind,
  description: Box<str>,
}

impl MessagingError {
  pub fn new<S: ToString>(kind: MessagingErrorKind, description: S) -> Self {
    MessagingError {
      kind,
      description: description.to_string().into_boxed_str(),
    }
  }

  pub fn no_accessor() -> Self {
    Self::new(MessagingErrorKind::NoAccessor, "No mailbox accessor")
  }

  pub fn not_found<I: Display>(index_type: &str, index: I) -> Self {
    Self::new(
      MessagingErrorKind::NotFound,
      format!("Mailbox index {} (type {}) not found", index, index_type),
    )
  }

  pub fn operation_not_supported(operation: &str) -> Self {
    Self::new(
      MessagingErrorKind::OperationNotSupported,
      format!("Mailbox operation not supported: {}", operation),
    )
  }

  pub fn already_exists<N: Display>(thing: &str, name: N) -> Self {
    Self::new(
      MessagingErrorKind::AlreadyExists,
      format!("{} {} already exists", thing, name),
    )
  }

  pub fn into_future<'a, T: 'a>(self) -> Box<Future<Item = T, Error = Self> + 'a> {
    Box::new(future::result(Err(self)))
  }
}

impl fmt::Display for MessagingError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", &self.description)
  }
}

impl Error for MessagingError {
  fn description(&self) -> &str {
    &self.description
  }
}
