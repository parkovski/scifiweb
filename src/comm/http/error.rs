use hyper::{self, Response, StatusCode};
use hyper::header::{ContentLength, ContentType};
use futures::future;
use comm::router::builder;
use super::RouteFuture;

pub struct ErrorHandler;
impl<'a> builder::ErrorHandler<'a, hyper::Error> for ErrorHandler {
  type Future = RouteFuture<'a>;

  fn on_error(&self, error: hyper::Error) -> RouteFuture<'a> {
    let message = format!("Server error: {}", error);
    Box::new(future::ok(
      Response::new()
        .with_header(ContentLength(message.len() as u64))
        .with_header(ContentType::plaintext())
        .with_status(StatusCode::InternalServerError)
        .with_body(message)
    ))
  }

  fn on_not_found(&self, path: &str) -> RouteFuture<'a> {
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