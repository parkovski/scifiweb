extern crate hyper;
extern crate futures;
#[macro_use]
extern crate log;
extern crate scifi_model as model;
extern crate scifi_router as router;
extern crate scifi_util as util;

use std::sync::Arc;
use hyper::{Request, Response};
use hyper::server::Http;
use model::access::ClonableAccessor;
use router::Rejection;
use router::hyper_router::HyperRouter;
use util::future::SFFuture;

mod error;
use self::error::ErrorHandler;
mod routes;
use self::routes::setup_routes;

pub type RouteFuture = SFFuture<'static, Response, error::Error>;
pub type FilterFuture = SFFuture<'static, (), Rejection<Response, error::Error>>;
pub type Router = router::Router<'static, Request, RouteFuture, FilterFuture, ErrorHandler>;

pub fn start<A: ClonableAccessor<'static> + 'static>(addr: &str, accessor: A) -> hyper::Result<()> {
  let router = Arc::new(HyperRouter::new(setup_routes(accessor)));
  let server = Http::new()
    .bind(&addr.parse().unwrap(), move || Ok(router.clone()))?;
  info!("Starting HTTP server for {}", addr);
  server.run()
}
