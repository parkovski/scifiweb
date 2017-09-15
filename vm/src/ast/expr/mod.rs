use std::sync::Arc;
use util::graph_cell::GraphRef;
use ast::SourceItem;
use ast::ty::Type;

mod primary;

lazy_static! {
  static ref EXPRESSION_SHARED_NAME: Arc<str> = "expression".into();
}

pub trait Expression<'a>: SourceItem {
  fn get_type(&self) -> GraphRef<'a, Type<'a>>;
}
