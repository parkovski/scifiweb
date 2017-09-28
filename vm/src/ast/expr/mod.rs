use std::fmt::{Debug, Display};
use serde::{Serialize, Serializer};
use erased_serde::Serialize as ErasedSerialize;
use util::graph_cell::GraphRef;
use util::cast::*;
use ast::SourceItem;
use ast::var::ScopeFilter;
use ast::ty::Type;

mod primary;
mod oper;

pub use self::primary::*;
pub use self::oper::*;

pub trait Expression<'a>
  : Debug
  + Display
  + ErasedSerialize
  + Cast<ErasedSerialize + 'a>
  + SourceItem
  + 'a
{
  fn ty(&self) -> GraphRef<'a, Type<'a>>;
  fn is_constant(&self) -> bool;
  fn precedence(&self) -> u8 { 0 }
  fn set_scope_filter(&mut self, filter: ScopeFilter<'a>) -> bool { false }
}

pub type BoxExpression<'a> = Box<Expression<'a> + 'a>;

impl<'a> Serialize for Expression<'a> {
  fn serialize<S: Serializer>(&self, serializer: S)
    -> ::std::result::Result<S::Ok, S::Error>
  {
    self.cast().serialize(serializer)
  }
}
