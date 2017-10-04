use ast::ty;

pub mod csharp;

/// Generates client code for types and stubs to call
/// events and functions. Notably, non-remote functions
/// are not generated because they don't authorize.
pub struct ClientCgVisitor<'ast> {
  fn visit_collectable(&self, c: &ty::Collectable<'ast>);
  fn visit_collectable_group(&self, c: &ty::CollectableGroup<'ast>);
  fn visit_object(&self, o: &ty::Object<'ast>);
  fn visit_user(&self, u: &ty::User<'ast>);
  fn visit_user_group(&self, u: &ty::UserGroup<'ast>);
  fn visit_event(&self, e: &ty::Event<'ast>);
  fn visit_remote_event(&self, e: &ty::RemoteEvent<'ast>);
  fn visit_remote_function(&self, f: &ty::RemoteFunction<'ast>);
}
