use std::sync::Arc;
use hyper::{self, Request, Response};
use hyper::server::Http;
use instance::access::Accessor;
use super::router::{self, Rejection};
use super::router::hyper::HyperRouter;
use util::future::SFFuture;

mod error;
use self::error::ErrorHandler;
mod routes;
use self::routes::setup_routes;

pub type RouteFuture<'a> = SFFuture<'a, Response, error::Error>;
pub type FilterFuture<'a> = SFFuture<'a, (), Rejection<Response, error::Error>>;
pub type Router<'a> = router::Router<'a, Request, RouteFuture<'a>, FilterFuture<'a>, ErrorHandler>;

pub fn start<A: Accessor<'static> + 'static>(addr: &str, accessor: A) -> hyper::Result<()> {
  let router = Arc::new(HyperRouter::new(setup_routes(accessor)));
  let server = Http::new().bind(&addr.parse().unwrap(), move || Ok(router.clone()))?;
  info!("Starting HTTP server for {}", addr);
  server.run()
}