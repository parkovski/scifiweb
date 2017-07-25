use std::sync::Arc;
use std::collections::HashMap;
use futures::future::{self, Future, Loop};
use route_recognizer::Router as Recognizer;
use url::Url;
use util::IntoBox;
use super::builder::{RouteEntry, FilterEntry, ErrorHandler};
use super::handlers::{Params, Rejection};

pub trait RoutePath {
  fn route_path(&self) -> &str;
}

/// Don't make a copy of the path
/// if it can be retrieved from the
/// request.
trait GetRoutePath<Rq> {
  fn new(path: &str) -> Self;
  fn get<'a>(&'a self, req: &'a Rq) -> &'a str;
}
struct SavedRoutePath(String);
struct UnsavedRoutePath;

impl<Rq> GetRoutePath<Rq> for SavedRoutePath {
  fn new(path: &str) -> Self {
    SavedRoutePath(path.to_string())
  }

  fn get<'a>(&'a self, _req: &'a Rq) -> &'a str {
    self.0.as_str()
  }
}

impl<Rq: RoutePath> GetRoutePath<Rq> for UnsavedRoutePath {
  fn new(_path: &str) -> Self {
    UnsavedRoutePath
  }

  fn get<'a>(&'a self, req: &'a Rq) -> &'a str {
    req.route_path()
  }
}

pub struct Router<'a, Rq, RFut, FFut, EH>
  where RFut: Future + 'a,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>> + 'a,
        EH: ErrorHandler<'a, RFut::Error> + 'a,
        EH::Future: Future<Item=RFut::Item> + 'a,
{
  recognizer: Recognizer<u32>,
  routes: Arc<Vec<RouteEntry<'a, Rq, RFut>>>,
  filters: Arc<Vec<FilterEntry<'a, Rq, RFut::Item, RFut::Error, FFut>>>,
  error_handler: Arc<EH>,
}

const ERROR_PARAM_REF_BUG: &'static str = "Bug: all other references to params should have been dropped";

impl<'a, Rq, RFut, FFut, EH> Router<'a, Rq, RFut, FFut, EH>
  where Rq: 'a,
        RFut: Future + 'a,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>> + 'a,
        EH: ErrorHandler<'a, RFut::Error> + 'a,
        EH::Future: Future<Item=RFut::Item> + 'a,
{
  pub(in super) fn new(
    recognizer: Recognizer<u32>,
    routes: Arc<Vec<RouteEntry<'a, Rq, RFut>>>,
    filters: Arc<Vec<FilterEntry<'a, Rq, RFut::Item, RFut::Error, FFut>>>,
    error_handler: EH
  ) -> Self
  {
    Router {
      recognizer,
      routes,
      filters,
      error_handler: Arc::new(error_handler),
    }
  }

  fn run_for_handler<GRP: GetRoutePath<Rq> + 'a>(
    &self,
    index: u32,
    req: Rq,
    path: &str,
    mut params: Params
  ) -> Box<Future<Item=RFut::Item, Error=<EH::Future as Future>::Error> + 'a>
  {
    let route = &self.routes[index as usize];
    let num_filters = route.filter_indexes.len();
    let error_handler = self.error_handler.clone();
    let mut ext = HashMap::new();
    
    // Add all the query parameters to params, beginning with a "?".
    // (ex. /foo?first=hello&second=goodbye => ?first: hello, ?second: goodbye)
    if let Ok(url) = Url::parse(path) {
      for (k, v) in url.query_pairs().into_iter() {
        let k = String::from("?") + &k;
        params.insert(k, v.into_owned());
      }
    }

    if num_filters == 0 {
      route.handler
        .call(req, &params, &mut ext)
        .or_else(move |err| error_handler.on_error(err))
        .into_box()
    } else {
      let get_path = GRP::new(path);
      let routes = self.routes.clone();
      let filters = self.filters.clone();
      let filter_indexes = route.filter_indexes.clone();
      let route_params = Box::new((req, params, ext));
      let filter_error_handler = error_handler.clone();
      let max_index = num_filters - 1;

      future::loop_fn((0, route_params), move |(index, mut route_params)| {
        {
          let (ref req, ref params, ref mut ext) = *route_params.as_mut();
          filters[filter_indexes[index] as usize].handler
            .call(req, params, ext)
        }
            .then(move |result| match result {
              // forward params to the route handler
              Ok(()) => Ok(if index < max_index {
                  Loop::Continue((index + 1, route_params))
                } else {
                  Loop::Break(route_params)
                }),
              Err(e) => Err((e, route_params)),
            })
      }).then(move |result| -> Box<Future<Item=RFut::Item, Error=<EH::Future as Future>::Error> + 'a> {
        match result {
          Err((Rejection::Response(res), _))
            => future::ok(res).into_box(),
          Err((Rejection::Error(e), _))
            => filter_error_handler.on_error(e).into_box(),
          Err((Rejection::NotFound, params))
            => filter_error_handler.on_not_found(get_path.get(&params.0)).into_box(),
          Ok(route_params) => {
            let unbox = *route_params;
            let (req, params, mut ext) = unbox;
            routes[index as usize].handler
              .call(req, &params, &mut ext)
              .or_else(move |err| filter_error_handler.on_error(err))
              .into_box()
          }
        }
      }).into_box()
    }
  }

  pub fn run_for_path(&self, path: &str, req: Rq)
  -> Box<Future<Item=RFut::Item, Error=<EH::Future as Future>::Error> + 'a>
  {
    let match_ = match self.recognizer.recognize(path) {
      Ok(m) => m,
      Err(_) => return self.error_handler.on_not_found(path).into_box(),
    };
    let index = *match_.handler;
    self.run_for_handler::<SavedRoutePath>(index, req, path, match_.params)
  }
}

impl<'a, Rq, RFut, FFut, EH> Router<'a, Rq, RFut, FFut, EH>
  where Rq: RoutePath + 'a,
        RFut: Future + 'a,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>> + 'a,
        EH: ErrorHandler<'a, RFut::Error> + 'a,
        EH::Future: Future<Item=RFut::Item> + 'a,
{
  pub fn run(&self, req: Rq) -> Box<Future<Item=RFut::Item, Error=<EH::Future as Future>::Error> + 'a> {
    let match_ = match self.recognizer.recognize(req.route_path()) {
      Ok(m) => m,
      Err(_) => return self.error_handler.on_not_found(req.route_path()).into_box(),
    };
    let index = *match_.handler;
    // The path parameter won't be used because it will be
    // fetched from the request if needed.
    self.run_for_handler::<UnsavedRoutePath>(index, req, "", match_.params)
  }
}