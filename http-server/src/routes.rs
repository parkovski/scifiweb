use hyper::{Request, Response, StatusCode};
use hyper::header::{ContentLength, ContentType};
use futures::Future;
use sf_router::{builder, ExtMap, GetAny, GetParam, Params};
use sf_router::hyper_router::{CommonMethods, SharedMethodFilters};
use sf_model::access::ClonableAccessor;
use sf_model::instance::Target;
use sf_model::instance::messaging::MessageLimit;
use sf_util::future::SFFuture;
use sf_util::Pipe;
use super::{FilterFuture, RouteFuture, Router};
use super::error::ErrorHandler;

type RouterBuilder = builder::RouterBuilder<
  'static,
  Request,
  RouteFuture,
  FilterFuture,
  ErrorHandler,
>;
type DirBuilder<P> = builder::DirBuilder<
  'static,
  Request,
  RouteFuture,
  FilterFuture,
  ErrorHandler,
  P,
>;

fn response(content_type: ContentType, body: &str) -> Response {
  Response::new()
    .with_header(ContentLength(body.len() as u64))
    .with_header(content_type)
    .with_status(StatusCode::Ok)
    .with_body(body.to_owned())
}

fn response_ok(body: &str) -> RouteFuture {
  Ok(response(ContentType::plaintext(), body)).pipe(SFFuture::new)
}

pub fn setup_routes<A: ClonableAccessor<'static> + 'static>(accessor: A) -> Router {
  let mut builder = RouterBuilder::new(ErrorHandler);
  let methods = SharedMethodFilters::new(&mut builder, |result| result.pipe(SFFuture::new));
  let common_methods = methods.common_methods();

  builder = builder.with_filter(move |_: &_, _: &_, ext: &mut ExtMap| -> FilterFuture {
    ext.insert("accessor".to_owned(), Box::new(accessor.clone()));
    Ok(()).pipe(SFFuture::new)
  });

  builder = setup_mailbox_routes::<_, A>(builder.dir("/messaging"), common_methods);
  builder = setup_event_routes::<_, A>(builder.dir("/event"), common_methods);

  builder.build()
}

/// /messaging/*
fn setup_mailbox_routes<P, A: ClonableAccessor<'static> + 'static>(
  builder: DirBuilder<P>,
  methods: &CommonMethods,
) -> RouterBuilder
{
  builder
    .dir("/mailbox")
      .route("/new", |_, params: &Params, ext: &mut ExtMap| -> RouteFuture {
        let accessor = ext.get_any::<A>("accessor").unwrap();
        let name = params.get_str_param("?name")?;
        let target = params.get_param::<Target>("?target")?;
        let message_limit = params.get_param::<MessageLimit>("message_limit")?;
        let thread_limit = params.get_param::<u32>("thread_limit")?;
        accessor.create_mailbox(target, name, message_limit, thread_limit)
          .map_err(From::from)
          .and_then(|mailbox| response_ok(format!("Created mailbox {}", mailbox.id()).as_str()))
          .pipe(SFFuture::new)
      })
      //.with_filter(methods.post())

      .route("/:name/for/:owner", |_, params: &Params, ext: &mut ExtMap| -> RouteFuture {
        let accessor = ext.get_any::<A>("accessor").unwrap();
        let name = params.get_str_param("name")?;
        let owner = params.get_param::<Target>("owner")?;
        accessor.get_mailbox_for_owner(owner, name)
          .map_err(From::from)
          .and_then(|mailbox| response_ok(format!("Got mailbox {}", mailbox.id()).as_str()))
          .pipe(SFFuture::new)
      })
      .with_filter(methods.get())

      .route("/:id", |_, _: &_, _: &mut _| -> RouteFuture {
        Ok(response(ContentType::plaintext(), "Mailbox by ID")).pipe(SFFuture::new)
      })
      .with_filter(methods.get())

      .route("/all/:owner", |_, _: &_, _: &mut _| -> RouteFuture {
        Ok(response(ContentType::plaintext(), "All mailboxes for owner")).pipe(SFFuture::new)
      })
      .with_filter(methods.get())

      .route("/:name/for/:owner/delete", |_, _: &_, _: &mut _| -> RouteFuture {
        Ok(response(ContentType::plaintext(), "Delete mailbox for owner")).pipe(SFFuture::new)
      })
      .with_filter(methods.post())

      .route("/:id/delete", |_, _: &_, _: &mut _| -> RouteFuture {
        Ok(response(ContentType::plaintext(), "Delete mailbox by ID")).pipe(SFFuture::new)
      })
      .with_filter(methods.post())

      .route("/all/:owner/delete", |_, _: &_, _: &mut _| -> RouteFuture {
        Ok(response(ContentType::plaintext(), "Delete all mailboxes for owner")).pipe(SFFuture::new)
      })
      .with_filter(methods.post())

      .route("/test", move |_, _: &_, ext: &mut ExtMap| -> RouteFuture {
        let accessor = ext.get_any::<A>("accessor").unwrap();
        accessor.create_message(0, Target::Global, "test", None, None)
          .map_err(From::from)
          .and_then(|message| response_ok(format!("created message {}", message.id()).as_str()))
          .pipe(SFFuture::new)
      })

    .to_root()
}

/// /event/*
fn setup_event_routes<P, A: ClonableAccessor<'static> + 'static>(
  builder: DirBuilder<P>,
  methods: &CommonMethods,
) -> RouterBuilder
{
  builder
    .route("/new/:template", |_, params: &Params, _: &mut _| -> RouteFuture {
      let template = params.get_str_param("template").unwrap();
      response_ok(format!("making event from template {}", template).as_str())
    })
    .with_filter(methods.post())

    .to_root()
}
