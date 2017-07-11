use std::net::Ipv4Addr;

use futures::Future;
use futures::future::{self, FutureResult};
use hyper;
use hyper::header::{ContentLength, ContentType};
use hyper::server::{Http, Service, Request, Response};

use super::router::{Router, Handler, Params, HyperRouter, ErrorHandler};
use instance::access::Accessor;

struct Hello;

impl Service for Hello {
  type Request = Request;
  type Response = Response;
  type Error = hyper::Error;
  type Future = FutureResult<Response, hyper::Error>;

  fn call(&self, _req: Request) -> Self::Future {
    future::ok(
      Response::new()
        .with_header(ContentLength("Hello".len() as u64))
        .with_header(ContentType::plaintext())
        .with_body("Hello")
    )
  }
}

impl ErrorHandler<FutureResult<Response, hyper::Error>> for Hello {
  fn on_error(&mut self, _error: hyper::Error) -> FutureResult<Response, hyper::Error> {
    future::ok(Response::new())
  }

  fn on_not_found(&mut self, _path: &str) -> FutureResult<Response, hyper::Error> {
    future::ok(Response::new())
  }
}

fn fut(text: &str) -> FutureResult<Response, hyper::Error> {
  future::ok(
    Response::new()
      .with_header(ContentLength(text.len() as u64))
      .with_header(ContentType::plaintext())
      .with_body(text.to_string())
  )
}

fn create_router<'a>() -> HyperRouter<'a, FutureResult<Response, hyper::Error>, FutureResult<(), Response>, Hello> {
  let mut router = Router::new(Hello);
  router.add("/", |_, _: &Params| fut("hello world"));
  HyperRouter::new(router)
}

pub fn start<'a, A: Accessor<'a> + 'a>(port: u16, _accessor: A) -> hyper::Result<()> {
  let addr = (Ipv4Addr::new(127, 0, 0, 1), port).into();
  let server = Http::new().bind(&addr, || Ok(Hello))?;
  server.run()
}