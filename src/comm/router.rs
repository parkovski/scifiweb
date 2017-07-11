use std::rc::Rc;
use std::cell::{Cell, RefCell};

use route_recognizer::Router as Recognizer;
use route_recognizer::Match;
pub use route_recognizer::Params;
use futures::{future, Future};

pub trait Handler<Request> {
  type Response;
  type Error;
  type Future: Future<Item=Self::Response, Error=Self::Error>;

  fn call(&mut self, req: Request, params: &Params) -> Self::Future;
}

impl<'a, F, Fut, Rq, Rs, E> Handler<Rq> for F
  where F: for<'r> FnMut(Rq, &Params) -> Fut,
        Fut: Future<Item=Rs, Error=E> + 'a
{
  type Response = Rs;
  type Error = E;
  type Future = Fut;

  fn call(&mut self, req: Rq, params: &Params) -> Self::Future {
    self(req, params)
  }
}

/// A successful filter just calls the next filter or
/// handler; a failed filter provides a response.
pub trait FilterHandler<Request> {
  type Response;
  type Future: Future<Item=(), Error=Self::Response>;

  fn call(&mut self, req: &Request, params: &Params) -> Self::Future;
}

impl<'a, F, Fut, Rq, Rs> FilterHandler<Rq> for F
  where F: for<'r> FnMut(&Rq, &Params) -> Fut,
        Fut: Future<Item=(), Error=Rs> + 'a
{
  type Response = Rs;
  type Future = Fut;

  fn call(&mut self, req: &Rq, params: &Params) -> Self::Future {
    self(req, params)
  }
}

/// Note: Returning an error from the error handler will
/// cause a panic.
pub trait ErrorHandler<Fut> where Fut: Future {
  fn on_error(&mut self, error: Fut::Error) -> Fut;
  fn on_not_found(&mut self, path: &str) -> Fut;
}

impl<'a, E, F, G, Rs, Fut> ErrorHandler<Fut> for (F, G)
  where F: for<'r> FnMut(E) -> Fut,
        G: for<'r> FnMut(&str) -> Fut,
        Fut: Future<Item=Rs, Error=E> + 'a
{
  fn on_error(&mut self, error: E) -> Fut {
    self.0(error)
  }

  fn on_not_found(&mut self, path: &str) -> Fut {
    self.1(path)
  }
}

impl<'a, E, F, Rs, Fut> ErrorHandler<Fut> for F
  where F: for<'r> FnMut(::either::Either<E, &str>) -> Fut,
        Fut: Future<Item=Rs, Error=E> + 'a
{
  fn on_error(&mut self, error: E) -> Fut {
    self(::either::Either::Left(error))
  }

  fn on_not_found(&mut self, path: &str) -> Fut {
    self(::either::Either::Right(path))
  }
}

struct HandlerEntry<'a, Rq, Rs, E, Fut> {
  pub handler: Box<Handler<Rq, Response=Rs, Error=E, Future=Fut> + 'a>,
  pub filter_index: Option<u32>,
}

struct FilterHandlerEntry<'a, Rq, Rs, Fut> {
  pub handler: Box<FilterHandler<Rq, Response=Rs, Future=Fut> + 'a>,
  pub previous_filter_index: Option<u32>,
}

pub struct FilterBuilder<'rt, 'fb, Rq, Rs, E, HFut, FFut, EH>
  where 'rt: 'fb,
        Rq: 'rt,
        Rs: 'rt,
        E: 'rt,
        HFut: Future<Item=Rs, Error=E> + 'rt,
        FFut: Future<Item=(), Error=Rs> + 'rt,
        EH: ErrorHandler<HFut> + 'rt,
{
  router: Option<&'fb mut Router<'rt, Rq, Rs, E, HFut, FFut, EH>>,
  base_path: String,
  filter_handler_index: u32,
  parent: Option<&'fb mut FilterBuilder<'rt, 'fb, Rq, Rs, E, HFut, FFut, EH>>,
}

impl<'rt, 'fb, Rq, Rs, E, HFut, FFut, EH> FilterBuilder<'rt, 'fb, Rq, Rs, E, HFut, FFut, EH>
  where 'rt: 'fb,
        HFut: Future<Item=Rs, Error=E>,
        FFut: Future<Item=(), Error=Rs>,
        EH: ErrorHandler<HFut>,
{
  fn fix_path(&self, subpath: &str) -> String {
    if subpath.len() == 0 {
      return self.base_path.clone();
    }
    let end_slash = self.base_path.ends_with('/');
    let start_slash = subpath.as_bytes()[0] == b'/';
    match (end_slash, start_slash) {
      (true, true) => self.base_path.clone() + &subpath[1..],
      (false, false) => self.base_path.clone() + "/" + subpath,
      _ => self.base_path.clone() + subpath,
    }
  }

  pub fn subdir<H>(&'fb mut self, subpath: &str, handler: H) -> FilterBuilder<'rt, 'fb, Rq, Rs, E, HFut, FFut, EH>
    where H: FilterHandler<Rq, Response=Rs, Future=FFut> + 'rt
  {
    let path = self.fix_path(subpath);
    let router = self.router.take().unwrap();
    router.subdir(Some(self), path, handler)
  }

  pub fn up(self) -> &'fb mut FilterBuilder<'rt, 'fb, Rq, Rs, E, HFut, FFut, EH> {
    let parent = self.parent.unwrap();
    parent.router = self.router;
    parent
  }

  pub fn done(&'fb mut self) -> &'fb mut Router<'rt, Rq, Rs, E, HFut, FFut, EH> {
    self.router.take().unwrap()
  }

  pub fn add<H>(&mut self, subpath: &str, handler: H) -> &mut Self
    where H: Handler<Rq, Response=Rs, Error=E, Future=HFut> + 'rt
  {
    let path = self.fix_path(subpath);
    self.router.as_mut().unwrap().add_with_filter(path.as_str(), Some(self.filter_handler_index), handler);
    self
  }
}

pub trait RoutePath {
  fn route_path(&self) -> &str;
}

impl RoutePath for ::hyper::server::Request {
  fn route_path(&self) -> &str {
    self.path()
  }
}

impl RoutePath for ::ws::Request {
  fn route_path(&self) -> &str {
    self.resource()
  }
}

trait IntoBoxFuture<F: Future> {
  fn into_box_future(self) -> Box<F>;
}

impl<'a, F: Future + 'a> IntoBoxFuture<F> for Box<F> {
  fn into_box_future(self) -> Box<F> {
    self
  }
}

impl<'a, F: Future + 'a> IntoBoxFuture<F> for F {
  fn into_box_future(self) -> Box<F> {
    Box::new(self)
  }
}

pub trait RouterRun<'a, Rq: RoutePath + 'a> {
  type Response;
  type Error;
  fn run(&self, req: Rq) -> Box<Future<Item=Self::Response, Error=Self::Error> + 'a>;
}

pub trait RouterRunForPath<'a, Rq: 'a> {
  type Response;
  type Error;
  fn run_for_path(&self, path: &str, req: Rq) -> Box<Future<Item=Self::Response, Error=Self::Error> + 'a>;
}

pub struct Router<'a, Rq, Rs, E, HFut, FFut, EH>
  where HFut: Future,
        EH: ErrorHandler<HFut>,
{
  recognizer: Recognizer<(u32, Option<u32>)>,
  handlers: Rc<Vec<RefCell<HandlerEntry<'a, Rq, Rs, E, HFut>>>>,
  filters: Rc<Vec<RefCell<FilterHandlerEntry<'a, Rq, Rs, FFut>>>>,
  error_handler: RefCell<EH>,
}

const ERROR_MODIFY_WHILE_RUNNING: &'static str = "The router cannot be modified while it is running";

impl<'a, Rq, Rs, E, HFut, FFut, EH> Router<'a, Rq, Rs, E, HFut, FFut, EH>
  where Rq: 'a,
        Rs: 'a,
        E: 'a,
        HFut: Future<Item=Rs, Error=E> + 'a,
        FFut: Future<Item=(), Error=Rs> + 'a,
        EH: ErrorHandler<HFut> + 'a,
{
  pub fn new(error_handler: EH) -> Self {
    Router {
      recognizer: Recognizer::new(),
      handlers: Rc::new(Vec::new()),
      filters: Rc::new(Vec::new()),
      error_handler: RefCell::new(error_handler),
    }
  }

  fn run_filter(
    &self,
    index: u32,
    shared_params: Rc<RefCell<(Rq, u32, Params)>>,
  ) -> Box<Future<Item=(), Error=Rs> + 'a>
  {
    let filters = self.filters.clone();
    let filter = &self.filters[index as usize];
    let previous_filter_index = filter.borrow().previous_filter_index.clone();
    if let Some(prev) = previous_filter_index {
      Box::new(self.run_filter(prev, shared_params.clone())
        .and_then(move |_| {
          let (ref request, _, ref params) = *shared_params.borrow();
          let result = filters[index as usize].borrow_mut().handler.call(request, params);
          result
        })
      )
    } else {
      let (ref request, _, ref params) = *shared_params.borrow();
      filter.borrow_mut().handler.call(request, params).into_box_future()
    }
  }

  fn run_for_handler(
    &self,
    req: Rq,
    (handler_index, filter_index): (u32, Option<u32>),
    params: Params
  ) -> Box<Future<Item=Rs, Error=E> + 'a>
  {
    let handlers = self.handlers.clone();
    if let Some(filter_index) = filter_index {
      let shared_params = Rc::new(RefCell::new((req, filter_index, params)));
      self.run_filter(filter_index, shared_params.clone())
        .then(move |result| -> Box<Future<Item=Rs, Error=E> + 'a> {
          if let Err(err) = result {
            return future::ok(err).into_box_future();
          }
          let (request, _, params) = Rc::try_unwrap(shared_params)
            .map_err(|_| "All filter references should already have been dropped")
            .unwrap()
            .into_inner();
          let response = handlers[handler_index as usize].borrow_mut().handler.call(request, &params).into_box_future();
          response
        })
        .into_box_future()
    } else {
      handlers[handler_index as usize].borrow_mut().handler.call(req, &params).into_box_future()
    }
  }

  pub(in self) fn subdir<'b, H>(
    &'b mut self,
    parent_builder: Option<&'b mut FilterBuilder<'a, 'b, Rq, Rs, E, HFut, FFut, EH>>,
    path: String,
    handler: H
  ) -> FilterBuilder<'a, 'b, Rq, Rs, E, HFut, FFut, EH>
    where H: FilterHandler<Rq, Response=Rs, Future=FFut> + 'a
  {
    let handler_index = {
      let mut filters = Rc::get_mut(&mut self.filters).expect(ERROR_MODIFY_WHILE_RUNNING);
      filters.push(RefCell::new(FilterHandlerEntry {
        handler: Box::new(handler),
        previous_filter_index: parent_builder.as_ref().map(|b| b.filter_handler_index),
      }));
      filters.len() - 1
    };
    FilterBuilder {
      router: Some(self),
      base_path: path,
      filter_handler_index: handler_index as u32,
      parent: parent_builder,
    }
  }

  pub(in self) fn add_with_filter<H>(&mut self, path: &str, filter_index: Option<u32>, handler: H)
    where H: Handler<Rq, Response=Rs, Error=E, Future=HFut> + 'a
  {
    let mut handlers = Rc::get_mut(&mut self.handlers).expect(ERROR_MODIFY_WHILE_RUNNING);
    let index = handlers.len();
    handlers.push(RefCell::new(HandlerEntry { handler: Box::new(handler), filter_index }));
    self.recognizer.add(path, (index as u32, filter_index));
  }

  pub fn filter<'b, H>(&'b mut self, path: &str, handler: H) -> FilterBuilder<'a, 'b, Rq, Rs, E, HFut, FFut, EH>
    where H: FilterHandler<Rq, Response=Rs, Future=FFut> + 'a
  {
    self.subdir(None, path.to_string(), handler)
  }

  pub fn add<H>(&mut self, path: &str, handler: H) -> &mut Self
    where H: Handler<Rq, Response=Rs, Error=E, Future=HFut> + 'a
  {
    self.add_with_filter(path, None, handler);
    self
  }
}

impl<'a, Rq, Rs, E, HFut, FFut, EH> RouterRun<'a, Rq> for Router<'a, Rq, Rs, E, HFut, FFut, EH>
  where Rq: RoutePath + 'a,
        Rs: 'a,
        E: 'a,
        HFut: Future<Item=Rs, Error=E> + 'a,
        FFut: Future<Item=(), Error=Rs> + 'a,
        EH: ErrorHandler<HFut> + 'a,
{
  type Response = Rs;
  type Error = E;

  fn run(&self, req: Rq) -> Box<Future<Item=Rs, Error=E> + 'a> {
    let match_ = match self.recognizer.recognize(req.route_path()) {
      Ok(m) => m,
      Err(_) => return self.error_handler.borrow_mut().on_not_found(req.route_path()).into_box_future(),
    };
    let indexes = match_.handler.clone();
    self.run_for_handler(req, indexes, match_.params)
  }
}

impl<'a, Rq, Rs, E, HFut, FFut, EH> RouterRunForPath<'a, Rq> for Router<'a, Rq, Rs, E, HFut, FFut, EH>
  where Rq: 'a,
        Rs: 'a,
        E: 'a,
        HFut: Future<Item=Rs, Error=E> + 'a,
        FFut: Future<Item=(), Error=Rs> + 'a,
        EH: ErrorHandler<HFut> + 'a,
{
  type Response = Rs;
  type Error = E;

  fn run_for_path(&self, path: &str, req: Rq) -> Box<Future<Item=Rs, Error=E> + 'a> {
    let match_ = match self.recognizer.recognize(path) {
      Ok(m) => m,
      Err(_) => return self.error_handler.borrow_mut().on_not_found(path).into_box_future(),
    };
    let indexes = match_.handler.clone();
    self.run_for_handler(req, indexes, match_.params)
  }
}

pub struct HyperRouter<'a, HFut, FFut, EH>
  where HFut: Future<Item=::hyper::Response, Error=::hyper::Error>,
        FFut: Future<Item=(), Error=::hyper::Response>,
        EH: ErrorHandler<HFut>,
{
  router: Router<'a, ::hyper::Request, ::hyper::Response, ::hyper::Error, HFut, FFut, EH>,
}

impl<'a, HFut, FFut, EH> HyperRouter<'a, HFut, FFut, EH>
  where HFut: Future<Item=::hyper::Response, Error=::hyper::Error>,
        FFut: Future<Item=(), Error=::hyper::Response>,
        EH: ErrorHandler<HFut>,
{
  pub fn new(
    router: Router<'a, ::hyper::Request, ::hyper::Response, ::hyper::Error, HFut, FFut, EH>
  ) -> Self
  {
    HyperRouter { router }
  }
}

impl<'a, HFut, FFut, EH> ::hyper::server::Service for HyperRouter<'a, HFut, FFut, EH>
  where HFut: Future<Item=::hyper::Response, Error=::hyper::Error> + 'a,
        FFut: Future<Item=(), Error=::hyper::Response> + 'a,
        EH: ErrorHandler<HFut> + 'a,
{
  type Request = ::hyper::Request;
  type Response = ::hyper::Response;
  type Error = ::hyper::Error;
  type Future = Box<Future<Item=::hyper::Response, Error=::hyper::Error> + 'a>;

  fn call(&self, req: ::hyper::server::Request) -> Self::Future {
    Box::new(self.router.run(req))
  }
}

#[cfg(test)]
mod test {
  use std::cell::Cell;
  use futures::future;
  use either::Either::{self, Left, Right};
  use super::*;

  struct AppendHandler(&'static str);

  impl<'a> Handler<Rc<Cell<String>>> for AppendHandler {
    type Response = ();
    type Error = ();
    type Future = future::FutureResult<(), ()>;
    fn call(&mut self, req: Rc<Cell<String>>, params: &Params) -> Self::Future {
      req.set(format!("{}: {:?}\n", self.0, params));
      future::ok(())
    }
  }

  struct Filter(bool);

  impl<'a> FilterHandler<Rc<Cell<String>>> for Filter {
    type Response = ();
    type Future = future::FutureResult<(), ()>;
    fn call(&mut self, _req: &Rc<Cell<String>>, _params: &Params) -> Self::Future {
      if self.0 { future::ok(()) } else { future::err(()) }
    }
  }

  #[test]
  fn test_router() {
    let mut output = String::new();
    let mut error_output = String::new();
    {
      let mut router = Router::new(|err: Either<(), &str>| {
        match err {
          Left(()) => error_output += "error\n",
          Right(path) => error_output += format!("not found: {}\n", path).as_str(),
        }
        future::ok(())
      });

      router
        .filter("/test", Filter(true))
          .subdir(":hi", Filter(true))
            .subdir("", Filter(true))
              .add("/foo", AppendHandler("/test/:hi/foo"))
        .done()
        .add("/:param/hi", AppendHandler("/:param/hi"))
        .filter("/foo", Filter(false))
          .add("/bar", AppendHandler("/foo/bar"));

      #[allow(unused_must_use)]
      {
        let paths = [
          "/test/foo", "/test/foo/foo", "/hello/hi",
          "/test/hi", "/foo/bar", "/notfound"
        ];
        for path in &paths {
          let out = Rc::new(Cell::new(String::new()));
          router.run_for_path(path, out.clone()).poll();
          output += out.take().as_str();
        }
      }
    }

    const EXPECTED: &'static str =
r#"/test/:hi/foo: Params { map: {"hi": "foo"} }
/:param/hi: Params { map: {"param": "hello"} }
/:param/hi: Params { map: {"param": "test"} }
-----
not found: /test/foo
not found: /notfound
"#;
    output += "-----\n";
    output += error_output.as_str();
    assert!(output == EXPECTED);
  }
}