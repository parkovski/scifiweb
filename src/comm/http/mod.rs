use std::sync::Arc;
use hyper::{self, Request, Response};
use hyper::server::Http;
use futures::Future;
use instance::access::Accessor;
use super::router::{self, Rejection};
use super::router::hyper::HyperRouter;

mod error;
use self::error::ErrorHandler;
mod routes;
use self::routes::setup_routes;

pub type RouteFuture<'a> = Box<Future<Item=Response, Error=hyper::Error> + 'a>;
pub type FilterFuture<'a> = Box<Future<Item=(), Error=Rejection<Response, hyper::Error>> + 'a>;
pub type Router<'a> = router::Router<'a, Request, RouteFuture<'a>, FilterFuture<'a>, ErrorHandler>;

pub fn start<'a, A: Accessor<'a> + 'a>(addr: &str, _accessor: A) -> hyper::Result<()> {
  let router = Arc::new(HyperRouter::new(setup_routes()));
  let server = Http::new().bind(&addr.parse().unwrap(), move || Ok(router.clone()))?;
  server.run()
}