use hyper::{Request, Response, StatusCode, Error as HyperError};
use hyper::header::{ContentType, ContentLength};
use futures::future::{self, IntoFuture};
use comm::router::{builder, ExtMap, get_any, Rejection};
use comm::router::hyper::{SharedMethodFilters, CommonMethods};
use super::{Router, RouteFuture, FilterFuture};
use super::error::ErrorHandler;
use util::IntoBox;

type RouterBuilder<'a> = builder::RouterBuilder<'a, Request, RouteFuture<'a>, FilterFuture<'a>, ErrorHandler>;
type DirBuilder<'a, P> = builder::DirBuilder<'a, Request, RouteFuture<'a>, FilterFuture<'a>, ErrorHandler, P>;

fn response(content_type: ContentType, body: &str) -> Response {
  Response::new()
    .with_header(ContentLength(body.len() as u64))
    .with_header(content_type)
    .with_status(StatusCode::Ok)
    .with_body(body.to_owned())
}

fn response_ok<'a>(body: &str) -> RouteFuture<'a> {
  future::ok(response(ContentType::plaintext(), body)).into_box()
}

pub fn setup_routes<'a>() -> Router<'a> {
  let mut builder = RouterBuilder::new(ErrorHandler);
  let methods = SharedMethodFilters::new(&mut builder, |result| Box::new(result.into_future()));

  builder = setup_mailbox_routes(builder.dir("/messaging"), methods.common_methods());

  builder.build()
}

/// /messaging/*
fn setup_mailbox_routes<'a, P>(
  builder: DirBuilder<'a, P>,
  methods: &CommonMethods
) -> RouterBuilder<'a>
{
  builder
    .dir("/mailbox")
      .route("/new", |_, _: &_, _: &mut _| -> RouteFuture<'a> {
        future::ok(response(ContentType::plaintext(), "New mailbox")).into_box()
      })
      .with_filter(methods.post())

      .route("/:name/for/:owner", |_, _: &_, _: &mut _| -> RouteFuture<'a> {
        future::ok(response(ContentType::plaintext(), "Mailbox for owner")).into_box()
      })
      .with_filter(methods.get())

      .route("/:id", |_, _: &_, _: &mut _| -> RouteFuture<'a> {
        future::ok(response(ContentType::plaintext(), "Mailbox by ID")).into_box()
      })
      .with_filter(methods.get())

      .route("/all/:owner", |_, _: &_, _: &mut _| -> RouteFuture<'a> {
        future::ok(response(ContentType::plaintext(), "All mailboxes for owner")).into_box()
      })
      .with_filter(methods.get())

      .route("/:name/for/:owner/delete", |_, _: &_, _: &mut _| -> RouteFuture<'a> {
        future::ok(response(ContentType::plaintext(), "Delete mailbox for owner")).into_box()
      })
      .with_filter(methods.post())

      .route("/:id/delete", |_, _: &_, _: &mut _| -> RouteFuture<'a> {
        future::ok(response(ContentType::plaintext(), "Delete mailbox by ID")).into_box()
      })
      .with_filter(methods.post())

      .route("/all/:owner/delete", |_, _: &_, _: &mut _| -> RouteFuture<'a> {
        future::ok(response(ContentType::plaintext(), "Delete all mailboxes for owner")).into_box()
      })
      .with_filter(methods.post())

    .to_root()
}