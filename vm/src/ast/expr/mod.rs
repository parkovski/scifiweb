use std::fmt::{Debug, Display};
use serde::{Serialize, Serializer};
use erased_serde::Serialize as ErasedSerialize;
use util::graph_cell::GraphRef;
use ast::SourceItem;
use ast::ty::Type;

mod primary;

pub use self::primary::*;

pub trait Expression<'a>
  : Debug
  + Display
  + ExpressionAsSerialize
  + SourceItem
  + 'a
{
  fn ty(&self) -> GraphRef<'a, Type<'a>>;
  fn is_constant(&self) -> bool;
}

pub type BoxExpression<'a> = Box<Expression<'a> + 'a>;

pub trait ExpressionAsSerialize {
  fn as_serialize(&self) -> &ErasedSerialize;
}

impl<T> ExpressionAsSerialize for T where T: ErasedSerialize {
  fn as_serialize(&self) -> &ErasedSerialize {
    self
  }
}

impl<'a> Serialize for Expression<'a> {
  fn serialize<S: Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
    self.as_serialize().serialize(serializer)
  }
}
