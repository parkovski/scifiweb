use hyper::{Request, Response, StatusCode};
use hyper::header::{ContentType, ContentLength};
use futures::future::{self, IntoFuture};
use comm::router::{builder, ExtMap, get_any};
use comm::router::hyper::CommonMethods;
use super::{Router, RouteFuture, FilterFuture};
use super::error::ErrorHandler;
use util::IntoBox;

type RouterBuilder<'a> = builder::RouterBuilder<'a, Request, RouteFuture<'a>, FilterFuture<'a>, ErrorHandler>;

fn response(content_type: ContentType, body: String) -> Response {
  Response::new()
    .with_header(ContentLength(body.len() as u64))
    .with_header(content_type)
    .with_status(StatusCode::Ok)
    .with_body(body)
}

pub fn setup_routes<'a>() -> Router<'a> {
  let mut builder = RouterBuilder::new(ErrorHandler);
  let methods = CommonMethods::new(&mut builder, |result| Box::new(result.into_future()));

  builder
    .route("/", |_req, _params: &_, ext: &mut _| -> RouteFuture<'a> {
      let any = get_any::<String>(ext, "Hello");
      let status = if any.is_some() { "found entry" } else { "didn't find" };
      future::ok(response(ContentType::plaintext(), status.to_string())).into_box()
    })
    .with_shared_filter(methods.get())
    .with_filter(|_req: &_, _params: &_, ext: &mut ExtMap| -> FilterFuture<'a> {
      ext.insert("Hello".into(), Box::new("world".to_string()));
      future::ok(()).into_box()
    })

    .route("/new", |_req, _params: &_, _ext: &mut _| -> RouteFuture<'a> {
      future::ok(response(ContentType::plaintext(), "Bye".into())).into_box()
    })
    .with_shared_filter(methods.post())

    .build()
}

fn setup_mailbox_routes<'a>(builder: RouterBuilder<'a>) -> RouterBuilder<'a> {
  builder
}