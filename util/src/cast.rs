use std::marker::Unsize;

pub trait ProvidesCast<U: ?Sized> {
  fn provide_cast(&self) -> &U;
  fn provide_cast_mut(&mut self) -> &mut U;
}

impl<T, U> ProvidesCast<U> for T
where
  T: Unsize<U>,
  U: ?Sized,
{
  fn provide_cast(&self) -> &U { self }
  fn provide_cast_mut(&mut self) -> &mut U { self }
}

pub trait Cast<U: ?Sized> {
  fn cast(&self) -> &U;
  fn cast_mut(&mut self) -> &mut U;
}

impl<T, U> Cast<U> for T
where
  T: ProvidesCast<U> + ?Sized,
  U: ?Sized,
{
  fn cast(&self) -> &U { self.provide_cast() }
  fn cast_mut(&mut self) -> &mut U { self.provide_cast_mut() }
}
