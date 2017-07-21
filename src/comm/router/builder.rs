use std::sync::Arc;
use futures::Future;
use route_recognizer::{Router as Recognizer};
use super::router::Router;
pub use super::handlers::{Route, Filter, ErrorHandler};
use super::Rejection;

pub(in super) struct RouteEntry<'a, Rq, Fut: Future + 'a> {
  pub handler: Box<Route<'a, Rq, Future=Fut> + 'a>,
  pub filter_indexes: Arc<Vec<u32>>,
}

pub(in super) struct FilterEntry<'a, Rq, Rs, E, Fut: Future<Item=(), Error=Rejection<Rs, E>> + 'a> {
  pub handler: Box<Filter<'a, Rq, Rs, E, Future=Fut> + 'a>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct FilterHandle(u32);
impl FilterHandle {
  pub fn new(id: u32) -> Self {
    FilterHandle(id)
  }
  pub fn id(&self) -> u32 {
    self.0
  }
}

/*
pub trait Builder<'a, Rq, RFut, FFut, EH>: Sized
where Rq: 'a,
      RFut: Future + 'a,
      FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>> + 'a,
      EH: ErrorHandler<'a, RFut::Error, Future=RFut> + 'a,
{
  /// Add a route. If inside a DirBuilder, this mounts
  /// the route inside that directory with any global filters
  /// on the router and any directory-level filters.
  fn route<R>(self, path: &str, handler: R) -> Self
    where R: Route<'a, Rq, Future=RFut> + 'a;
  
  /// Create a new filter not yet associated with any
  /// route or directory.
  fn new_filter<F>(&mut self, handler: F) -> FilterHandle
    where F: Filter<'a, Rq, RFut::Item, RFut::Error, Future=FFut> + 'a;
  
  /// Add a filter to the last route or directory created.
  /// If none was created yet, or a DirBuilder was just exited,
  /// this will add a global filter if at the top level, or a
  /// directory-level filter if in a DirBuilder.
  fn with_filter<F>(self, handler: F) -> Self
    where F: Filter<'a, Rq, RFut::Item, RFut::Error, Future=FFut> + 'a;

  /// Add a filter from a FilterHandle created with new_filter.
  /// This way, filters can be reused for multiple routes.
  fn with_shared_filter(self, handle: FilterHandle) -> Self;

  /// Enter a new directory. This is not necessary just to mount routes
  /// containing multiple sections (/foo/bar/baz) - the purpose of directories
  /// is to add filters that apply to all sub-routes within them,
  /// e.g. authenticate /admin/(*).
  fn dir(self, path: &str) -> DirBuilder<'a, Rq, RFut, FFut, EH, Self>;

  /// Create the router, consuming the builder.
  fn build(self) -> Router<'a, Rq, RFut, FFut, EH>;
}
*/

pub struct RouterBuilder<'a, Rq, RFut, FFut, EH>
  where RFut: Future + 'a,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>> + 'a,
        EH: ErrorHandler<'a, RFut::Error, Future=RFut> + 'a,
{
  recognizer: Recognizer<u32>,
  routes: Vec<RouteEntry<'a, Rq, RFut>>,
  filters: Vec<FilterEntry<'a, Rq, RFut::Item, RFut::Error, FFut>>,
  global_filters: Arc<Vec<u32>>,
  error_handler: EH,
  last_route_index: Option<u32>,
}

/// If this list is shared between routes, we need to
/// make this one unique to add a route to it.
fn add_index_unique(vec: &mut Arc<Vec<u32>>, index: u32) {
  if let Some(indexes) = Arc::get_mut(vec) {
    indexes.push(index);
    return;
  }
  let mut new_vec = Vec::clone(vec);
  new_vec.push(index);
  *vec = Arc::new(new_vec);
}

impl<'a, Rq, RFut, FFut, EH> RouterBuilder<'a, Rq, RFut, FFut, EH>
  where Rq: 'a,
        RFut: Future + 'a,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>> + 'a,
        EH: ErrorHandler<'a, RFut::Error, Future=RFut> + 'a,
{
  pub fn new(error_handler: EH) -> Self {
    RouterBuilder {
      recognizer: Recognizer::new(),
      routes: Vec::new(),
      filters: Vec::new(),
      global_filters: Arc::new(Vec::new()),
      error_handler,
      last_route_index: None,
    }
  }

  fn new_route<R>(&mut self, path: &str, filter_indexes: Arc<Vec<u32>>, handler: R) -> u32
  where R: Route<'a, Rq, Future=RFut> + 'a
  {
    let index = self.routes.len() as u32;
    let handler = Box::new(handler);
    self.routes.push(RouteEntry { handler, filter_indexes });
    self.recognizer.add(path, index);
    index
  }

  fn add_filter_to_route(&mut self, route_index: u32, filter_index: u32) {
    add_index_unique(&mut self.routes[route_index as usize].filter_indexes, filter_index);
  }

  pub fn route<R>(mut self, path: &str, handler: R) -> Self
    where R: Route<'a, Rq, Future=RFut> + 'a
  {
    let filter_indexes = self.global_filters.clone();
    let route_index = self.new_route(path, filter_indexes, handler);
    self.last_route_index = Some(route_index);
    self
  }

  pub fn new_filter<F>(&mut self, handler: F) -> FilterHandle
  where F: Filter<'a, Rq, RFut::Item, RFut::Error, Future=FFut> + 'a
  {
    let index = self.filters.len();
    self.filters.push(FilterEntry { handler: Box::new(handler) });
    FilterHandle::new(index as u32)
  }

  pub fn with_filter<F>(mut self, handler: F) -> Self
    where F: Filter<'a, Rq, RFut::Item, RFut::Error, Future=FFut> + 'a
  {
    let filter_handle = self.new_filter(handler);
    match self.last_route_index {
      Some(route) => {
        self.add_filter_to_route(route, filter_handle.id());
      }
      None => {
        add_index_unique(&mut self.global_filters, filter_handle.id());
      }
    }
    self
  }

  pub fn with_shared_filter(mut self, filter_handle: FilterHandle) -> Self {
    match self.last_route_index {
      Some(route) => self.add_filter_to_route(route, filter_handle.id()),
      None => add_index_unique(&mut self.global_filters, filter_handle.id()),
    }
    self
  }

  pub fn dir(mut self, path: &str)
    -> DirBuilder<'a, Rq, RFut, FFut, EH, Self>
  {
    // After building a dir, it would be unintuitive
    // for filters to stick to the last route before that.
    self.last_route_index = None;
    let filter_indexes = self.global_filters.clone();
    DirBuilder {
      router_builder: Some(Box::new(self)),
      parent: None,
      base_path: path.to_string(),
      filter_indexes,
      last_route_index: None,
    }
  }

  pub fn build(self) -> Router<'a, Rq, RFut, FFut, EH> {
    Router::new(
      self.recognizer,
      Arc::new(self.routes),
      Arc::new(self.filters),
      self.error_handler
    )
  }
}

const ONLY_ACCESSIBLE_BUILDER_HAS_REF: &'static str
  = "This reference is always transfered to the only accessible DirBuilder";

pub struct DirBuilder<'a, Rq, RFut, FFut, EH, Par>
  where Rq: 'a,
        RFut: Future + 'a,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>> + 'a,
        EH: ErrorHandler<'a, RFut::Error, Future=RFut> + 'a,
        Par: Sized + 'a,
{
  router_builder: Option<Box<RouterBuilder<'a, Rq, RFut, FFut, EH>>>,
  parent: Option<Box<Par>>,
  base_path: String,
  filter_indexes: Arc<Vec<u32>>,
  last_route_index: Option<u32>,
}

impl<'a, Rq, RFut, FFut, EH, Par>
DirBuilder<'a, Rq, RFut, FFut, EH, Par>
  where RFut: Future,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>>,
        EH: ErrorHandler<'a, RFut::Error, Future=RFut>,
{
  /// Join two paths, converting either zero or
  /// two slashes to one at the join point.
  fn join_path(&self, subpath: &str) -> String {
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

  pub fn to_root(self) -> RouterBuilder<'a, Rq, RFut, FFut, EH> {
    *self.router_builder.expect(ONLY_ACCESSIBLE_BUILDER_HAS_REF)
  }

  pub fn route<R>(mut self, path: &str, handler: R) -> Self
  where R: Route<'a, Rq, Future=RFut> + 'a
  {
    let path = self.join_path(path);
    self.last_route_index = Some(
      self.router_builder.as_mut().expect(ONLY_ACCESSIBLE_BUILDER_HAS_REF).new_route(
        path.as_str(),
        self.filter_indexes.clone(),
        handler,
      )
    );
    self
  }

  pub fn new_filter<F>(&mut self, handler: F) -> FilterHandle
  where F: Filter<'a, Rq, RFut::Item, RFut::Error, Future=FFut> + 'a
  {
    self.router_builder.as_mut().expect(ONLY_ACCESSIBLE_BUILDER_HAS_REF).new_filter(handler)
  }

  pub fn with_filter<F>(mut self, handler: F) -> Self
  where F: Filter<'a, Rq, RFut::Item, RFut::Error, Future=FFut> + 'a
  {
    match (self.last_route_index, self.router_builder.as_mut()) {
      (Some(route), Some(router_builder)) => {
        let filter_handle = router_builder.new_filter(handler);
        router_builder.add_filter_to_route(route, filter_handle.id());
      }
      (None, Some(router_builder)) => {
        let filter_handle = router_builder.new_filter(handler);
        add_index_unique(&mut self.filter_indexes, filter_handle.id());
      }
      _ => unreachable!(),
    }
    self
  }

  pub fn with_shared_filter(mut self, filter_handle: FilterHandle) -> Self {
    match (self.last_route_index, self.router_builder.as_mut()) {
      (Some(route), Some(router_builder))
        => router_builder.add_filter_to_route(route, filter_handle.id()),
      (None, _)
        => add_index_unique(&mut self.filter_indexes, filter_handle.id()),
      _ => unreachable!(),
    }
    self
  }

  pub fn dir(mut self, path: &str) -> DirBuilder<'a, Rq, RFut, FFut, EH, Self> {
    let base_path = self.join_path(path);
    let router_builder = self.router_builder.take();
    let filter_indexes = self.filter_indexes.clone();
    DirBuilder {
      router_builder,
      parent: Some(Box::new(self)),
      base_path,
      filter_indexes,
      last_route_index: None,
    }
  }

  pub fn build(self) -> Router<'a, Rq, RFut, FFut, EH> {
    self.router_builder.expect(ONLY_ACCESSIBLE_BUILDER_HAS_REF).build()
  }
}

impl<'a, Rq, RFut, FFut, EH, Par>
DirBuilder<
  'a, Rq, RFut, FFut, EH,
  DirBuilder<'a, Rq, RFut, FFut, EH, Par>
>
where RFut: Future,
      FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>>,
      EH: ErrorHandler<'a, RFut::Error, Future=RFut>,
{
  pub fn up(self) -> DirBuilder<'a, Rq, RFut, FFut, EH, Par> {
    let mut parent = self.parent.expect(ONLY_ACCESSIBLE_BUILDER_HAS_REF);
    parent.router_builder = self.router_builder;
    *parent
  }
}

impl<'a, Rq, RFut, FFut, EH>
DirBuilder<
  'a, Rq, RFut, FFut, EH,
  RouterBuilder<'a, Rq, RFut, FFut, EH>
>
  where RFut: Future,
        FFut: Future<Item=(), Error=Rejection<RFut::Item, RFut::Error>>,
        EH: ErrorHandler<'a, RFut::Error, Future=RFut>,
{
  pub fn up(self) -> RouterBuilder<'a, Rq, RFut, FFut, EH> {
    *self.router_builder.expect(ONLY_ACCESSIBLE_BUILDER_HAS_REF)
  }
}