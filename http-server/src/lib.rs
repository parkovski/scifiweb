extern crate hyper;
extern crate futures;
#[macro_use]
extern crate log;
extern crate sf_model;
extern crate sf_router;
extern crate sf_util;

use std::sync::Arc;
use hyper::{Request, Response};
use hyper::server::Http;
use sf_model::access::Accessor;
use sf_router::Rejection;
use sf_router::hyper_router::HyperRouter;
use sf_util::future::SFFuture;

mod error;
use self::error::ErrorHandler;
mod routes;
use self::routes::setup_routes;

pub type RouteFuture = SFFuture<'static, Response, error::Error>;
pub type FilterFuture = SFFuture<'static, (), Rejection<Response, error::Error>>;
pub type Router = sf_router::Router<'static, Request, RouteFuture, FilterFuture, ErrorHandler>;

pub fn start<A: Accessor<'static> + 'static>(addr: &str, accessor: A) -> hyper::Result<()> {
  let router = Arc::new(HyperRouter::new(setup_routes(accessor)));
  let server = Http::new()
    .bind(&addr.parse().unwrap(), move || Ok(router.clone()))?;
  info!("Starting HTTP server for {}", addr);
  server.run()
}
