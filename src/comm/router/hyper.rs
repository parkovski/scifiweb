use std::sync::Arc;

use hyper::{Request, Response, Error as HyperError, Method};
use hyper::server::Service;
use futures::Future;

use super::{Params, Rejection, ExtMap};
use super::router::{Router, RoutePath};
use super::builder::{RouterBuilder, Filter, FilterHandle, ErrorHandler};

impl RoutePath for ::hyper::server::Request {
  fn route_path(&self) -> &str {
    self.path()
  }
}

pub struct HyperRouter<'a, RFut, FFut, EH>
where RFut: Future<Item=Response, Error=HyperError> + 'a,
      FFut: Future<Item=(), Error=Rejection<Response, HyperError>> + 'a,
      EH: ErrorHandler<'a, HyperError, Future=RFut> + 'a,
{
  router: Router<'a, Request, RFut, FFut, EH>,
}

impl<'a, RFut, FFut, EH> HyperRouter<'a, RFut, FFut, EH>
  where RFut: Future<Item=Response, Error=HyperError> + 'a,
        FFut: Future<Item=(), Error=Rejection<Response, HyperError>> + 'a,
        EH: ErrorHandler<'a, HyperError, Future=RFut> + 'a,
{
  pub fn new(
    router: Router<'a, Request, RFut, FFut, EH>
  ) -> Self
  {
    HyperRouter { router }
  }
}

impl<'a, RFut, FFut, EH> Service for HyperRouter<'a, RFut, FFut, EH>
  where RFut: Future<Item=Response, Error=HyperError> + 'a,
        FFut: Future<Item=(), Error=Rejection<Response, HyperError>> + 'a,
        EH: ErrorHandler<'a, HyperError, Future=RFut> + 'a,
{
  type Request = Request;
  type Response = Response;
  type Error = HyperError;
  type Future = Box<Future<Item=Response, Error=HyperError> + 'a>;

  fn call(&self, req: Request) -> Self::Future {
    self.router.run(req)
  }
}

struct MethodFilter<FFut, F>
where FFut: Future<Item=(), Error=Rejection<Response, HyperError>>,
      F: Fn(Result<(), Rejection<Response, HyperError>>) -> FFut +  Send + Sync,
{
  method: Method,
  make_future: Arc<F>,
}

impl<FFut, F> MethodFilter<FFut, F>
where FFut: Future<Item=(), Error=Rejection<Response, HyperError>>,
      F: Fn(Result<(), Rejection<Response, HyperError>>) -> FFut + Send + Sync,
{
  pub fn new(make_future: Arc<F>, method: Method) -> Self {
    MethodFilter { method, make_future }
  }
}

impl<'a, FFut, F> Filter<'a, Request, Response, HyperError> for MethodFilter<FFut, F>
where FFut: Future<Item=(), Error=Rejection<Response, HyperError>> + 'a,
      F: Fn(Result<(), Rejection<Response, HyperError>>) -> FFut + Send + Sync + 'a,
{
  type Future = FFut;

  fn call(&self, req: &Request, _params: &Params, _ext: &mut ExtMap) -> Self::Future {
    if req.method() == &self.method {
      (self.make_future)(Ok(()))
    } else {
      (self.make_future)(Err(Rejection::NotFound))
    }
  }
}

pub struct CommonMethods {
  get: FilterHandle,
  post: FilterHandle,
  put: FilterHandle,
  delete: FilterHandle,
}

impl CommonMethods {
  pub fn get(&self) -> FilterHandle {
    self.get
  }

  pub fn post(&self) -> FilterHandle {
    self.post
  }

  pub fn put(&self) -> FilterHandle {
    self.put
  }

  pub fn delete(&self) -> FilterHandle {
    self.delete
  }
}

pub struct SharedMethodFilters<FFut, F>
where FFut: Future<Item=(), Error=Rejection<Response, HyperError>>,
      F: Fn(Result<(), Rejection<Response, HyperError>>) -> FFut + Send + Sync,
{
  common_methods: CommonMethods,
  make_future: Arc<F>,
}

impl<'a, FFut, F> SharedMethodFilters<FFut, F>
where FFut: Future<Item=(), Error=Rejection<Response, HyperError>>,
      F: Fn(Result<(), Rejection<Response, HyperError>>) -> FFut + Send + Sync + 'a,
{
  pub fn new<RFut, EH>(
    builder: &mut RouterBuilder<'a, Request, RFut, FFut, EH>,
    make_future: F,
  ) -> Self
  where RFut: Future<Item=Response, Error=HyperError> + 'a,
        EH: ErrorHandler<'a, HyperError, Future=RFut> + 'a,
  {
    let make_future = Arc::new(make_future);
    SharedMethodFilters {
      common_methods: CommonMethods {
        get: builder.new_filter(MethodFilter::new(make_future.clone(), Method::Get)),
        post: builder.new_filter(MethodFilter::new(make_future.clone(), Method::Post)),
        put: builder.new_filter(MethodFilter::new(make_future.clone(), Method::Put)),
        delete: builder.new_filter(MethodFilter::new(make_future.clone(), Method::Delete)),
      },
      make_future,
    }
  }

  pub fn common_methods(&self) -> &CommonMethods {
    &self.common_methods
  }

  pub fn get(&self) -> FilterHandle {
    self.common_methods.get()
  }

  pub fn post(&self) -> FilterHandle {
    self.common_methods.post()
  }

  pub fn put(&self) -> FilterHandle {
    self.common_methods.put()
  }

  pub fn delete(&self) -> FilterHandle {
    self.common_methods.delete()
  }

  pub fn make_custom<RFut, EH>(
    &self,
    builder: &mut RouterBuilder<'a, Request, RFut, FFut, EH>,
    method: Method,
  ) -> FilterHandle
  where RFut: Future<Item=Response, Error=HyperError> + 'a,
        EH: ErrorHandler<'a, HyperError, Future=RFut> + 'a,
  {
    builder.new_filter(MethodFilter::new(self.make_future.clone(), method))
  }
}