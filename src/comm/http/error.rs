use std::fmt;
use std::error::Error as StdError;
use hyper::{self, Response, StatusCode};
use hyper::header::{ContentLength, ContentType};
use futures::{future, Future};
use comm::router::ParamError;
use comm::router::builder;
use instance::mailbox::MailboxError;

#[derive(Debug)]
pub enum Error {
  Mailbox(MailboxError),
  Param(ParamError),
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &Error::Mailbox(ref mberr) => mberr.fmt(f),
      &Error::Param(ref pnferr) => pnferr.fmt(f),
    }
  }
}

impl StdError for Error {
  fn description(&self) -> &str {
    match self {
      &Error::Mailbox(ref mberr) => mberr.description(),
      &Error::Param(ref pnferr) => pnferr.description(),
    }
  }
}

impl From<MailboxError> for Error {
  fn from(mailbox_error: MailboxError) -> Self {
    Error::Mailbox(mailbox_error)
  }
}

impl From<ParamError> for Error {
  fn from(param_error: ParamError) -> Self {
    Error::Param(param_error)
  }
}

pub struct ErrorHandler;
impl<'a> builder::ErrorHandler<'a, Error> for ErrorHandler {
  type Future = Box<Future<Item=Response, Error=hyper::Error> + 'a>;

  fn on_error(&self, error: Error) -> Self::Future {
    let message = format!("Server error: {}", error);
    Box::new(future::ok(
      Response::new()
        .with_header(ContentLength(message.len() as u64))
        .with_header(ContentType::plaintext())
        .with_status(StatusCode::InternalServerError)
        .with_body(message)
    ))
  }

  fn on_not_found(&self, path: &str) -> Self::Future {
    let message = format!("Not found: {}", path);
    Box::new(future::ok(
      Response::new()
        .with_header(ContentLength(message.len() as u64))
        .with_header(ContentType::plaintext())
        .with_status(StatusCode::NotFound)
        .with_body(message)
    ))
  }
}