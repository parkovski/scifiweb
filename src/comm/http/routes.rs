use hyper::{Request, Response, StatusCode};
use hyper::header::{ContentType, ContentLength};
use futures::Future;
use comm::router::{builder, Params, ExtMap, get_any, get_str_param, get_param};
use comm::router::hyper::{SharedMethodFilters, CommonMethods};
use instance::access::Accessor;
use instance::Target;
use instance::mailbox::MessageLimit;
use util::future::SFFuture;
use util::Pipe;
use super::{Router, RouteFuture, FilterFuture};
use super::error::ErrorHandler;

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
  Ok(response(ContentType::plaintext(), body)).pipe(SFFuture::new)
}

pub fn setup_routes<A: Accessor<'static> + 'static>(accessor: A) -> Router<'static> {
  let mut builder = RouterBuilder::new(ErrorHandler);
  let methods = SharedMethodFilters::new(&mut builder, |result| result.pipe(SFFuture::new));

  builder = builder.with_filter(move |_: &_, _: &_, ext: &mut ExtMap| -> FilterFuture<'static> {
    ext.insert("accessor".to_owned(), Box::new(accessor.clone()));
    Ok(()).pipe(SFFuture::new)
  });

  builder = setup_mailbox_routes::<_, A>(builder.dir("/messaging"), methods.common_methods());

  builder.build()
}

/// /messaging/*
fn setup_mailbox_routes<P, A: Accessor<'static> + 'static>(
  builder: DirBuilder<'static, P>,
  methods: &CommonMethods,
) -> RouterBuilder<'static>
{
  builder
    .dir("/mailbox")
      .route("/new", |_, params: &Params, ext: &mut _| -> RouteFuture<'static> {
        let accessor = get_any::<A>(ext, "accessor").unwrap();
        let name = get_str_param(params, "?name")?;
        let target = get_param::<Target>(params, "?target")?;
        let message_limit = get_param::<MessageLimit>(params, "message_limit")?;
        let thread_limit = get_param::<u32>(params, "thread_limit")?;
        accessor.create_mailbox(target, name, message_limit, thread_limit)
          .map_err(From::from)
          .and_then(|mailbox| response_ok(format!("Created mailbox {}", mailbox.id()).as_str()))
          .pipe(SFFuture::new)
      })
      //.with_filter(methods.post())

      .route("/:name/for/:owner", |_, _: &_, _: &mut _| -> RouteFuture<'static> {
        Ok(response(ContentType::plaintext(), "Mailbox for owner")).pipe(SFFuture::new)
      })
      .with_filter(methods.get())

      .route("/:id", |_, _: &_, _: &mut _| -> RouteFuture<'static> {
        Ok(response(ContentType::plaintext(), "Mailbox by ID")).pipe(SFFuture::new)
      })
      .with_filter(methods.get())

      .route("/all/:owner", |_, _: &_, _: &mut _| -> RouteFuture<'static> {
        Ok(response(ContentType::plaintext(), "All mailboxes for owner")).pipe(SFFuture::new)
      })
      .with_filter(methods.get())

      .route("/:name/for/:owner/delete", |_, _: &_, _: &mut _| -> RouteFuture<'static> {
        Ok(response(ContentType::plaintext(), "Delete mailbox for owner")).pipe(SFFuture::new)
      })
      .with_filter(methods.post())

      .route("/:id/delete", |_, _: &_, _: &mut _| -> RouteFuture<'static> {
        Ok(response(ContentType::plaintext(), "Delete mailbox by ID")).pipe(SFFuture::new)
      })
      .with_filter(methods.post())

      .route("/all/:owner/delete", |_, _: &_, _: &mut _| -> RouteFuture<'static> {
        Ok(response(ContentType::plaintext(), "Delete all mailboxes for owner")).pipe(SFFuture::new)
      })
      .with_filter(methods.post())

      .route("/test", move |_, _: &_, ext: &mut _| -> RouteFuture<'static> {
        let accessor = get_any::<A>(ext, "accessor").unwrap();
        accessor.create_message(0, Target::Global, "test", None, None)
          .map_err(From::from)
          .and_then(|message| response_ok(format!("created message {}", message.id()).as_str()))
          .pipe(SFFuture::new)
      })

    .to_root()
}