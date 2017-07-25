use std::collections::HashMap;
use std::any::Any;
use std::fmt;
use std::error::Error;
use std::str::FromStr;
use futures::Future;
use either::Either;
pub use route_recognizer::Params;

pub type ExtMap = HashMap<String, Box<Any>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamErrorKind {
  NotFound,
  InvalidConversion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamError {
  description: String,
  kind: ParamErrorKind,
}

impl ParamError {
  pub fn not_found(key_type: &'static str, key: &str) -> Self {
    ParamError {
      description: format!("{} \"{}\" not found", key_type, key),
      kind: ParamErrorKind::NotFound,
    }
  }

  pub fn invalid_conversion(value: &str) -> Self {
    ParamError {
      description: format!("Invalid conversion for \"{}\"", value),
      kind: ParamErrorKind::InvalidConversion,
    }
  }
}

impl fmt::Display for ParamError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.description.as_str())
  }
}

impl Error for ParamError {
  fn description(&self) -> &str {
    self.description.as_str()
  }
}

pub fn get_any<'a, T: 'static>(map: &'a ExtMap, key: &str) -> Result<&'a T, ParamError> {
  map.get(key).and_then(|any| any.downcast_ref()).ok_or_else(|| ParamError::not_found("Extension param", key))
}

pub fn get_any_mut<'a, T: 'static>(map: &'a mut ExtMap, key: &str) -> Result<&'a mut T, ParamError> {
  map.get_mut(key).and_then(|any| any.downcast_mut()).ok_or_else(|| ParamError::not_found("Extension param", key))
}

pub fn get_str_param<'a>(params: &'a Params, key: &str) -> Result<&'a str, ParamError> {
  if let Some(param) = params.find(key) {
    Ok(param)
  } else {
    Err(ParamError::not_found("route param", key))
  }
}

pub fn get_param<T: FromStr>(params: &Params, key: &str) -> Result<T, ParamError> {
  if let Some(param) = params.find(key) {
    param.parse::<T>().map_err(|_| ParamError::invalid_conversion(param))
  } else {
    Err(ParamError::not_found("route param", key))
  }
}

pub trait Route<'a, Rq>: Send + Sync {
  type Future: Future + 'a;

  fn call(&self, req: Rq, params: &Params, ext: &mut ExtMap) -> Self::Future;
}

/// TODO: Is there any way to implement this so you don't
/// have to list out the types in every closure?
impl<'a, Rq, F, Fut> Route<'a, Rq> for F
where F: Fn(Rq, &Params, &mut ExtMap) -> Fut + Send + Sync + 'a,
      Fut: Future + 'a,
{
  type Future = Fut;

  fn call(&self, req: Rq, params: &Params, ext: &mut ExtMap) -> Self::Future {
    self(req, params, ext)
  }
}

pub enum Rejection<Rs, E> {
  Response(Rs),
  Error(E),
  NotFound,
}

/// A successful filter just calls the next filter or
/// handler; a failed filter provides a response.
pub trait Filter<'a, Rq, Rs, E>: Send + Sync {
  type Future: Future<Item=(), Error=Rejection<Rs, E>>;

  fn call(&self, req: &Rq, params: &Params, ext: &mut ExtMap) -> Self::Future;
}

impl<'a, Rq, Rs, E, F, Fut> Filter<'a, Rq, Rs, E> for F
  where F: Fn(&Rq, &Params, &mut ExtMap) -> Fut + Send + Sync + 'a,
        Fut: Future<Item=(), Error=Rejection<Rs, E>> + 'a,
{
  type Future = Fut;

  fn call(&self, req: &Rq, params: &Params, ext: &mut ExtMap) -> Self::Future {
    self(req, params, ext)
  }
}

/// Note: Returning an error from the error handler will
/// cause the router to stop running.
pub trait ErrorHandler<'a, E> {
  type Future: Future + 'a;

  fn on_error(&self, error: E) -> Self::Future;
  fn on_not_found(&self, path: &str) -> Self::Future;
}

impl<'a, E, F, G, Fut> ErrorHandler<'a, E> for (F, G)
  where F: Fn(E) -> Fut,
        G: Fn(&str) -> Fut,
        Fut: Future + 'a,
{
  type Future = Fut;

  fn on_error(&self, error: E) -> Fut {
    self.0(error)
  }

  fn on_not_found(&self, path: &str) -> Fut {
    self.1(path)
  }
}

impl<'a, E, F, Fut> ErrorHandler<'a, E> for F
  where F: Fn(Either<E, &str>) -> Fut,
        Fut: Future + 'a,
{
  type Future = Fut;

  fn on_error(&self, error: E) -> Fut {
    self(Either::Left(error))
  }

  fn on_not_found(&self, path: &str) -> Fut {
    self(Either::Right(path))
  }
}