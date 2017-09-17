use std::fmt::{self, Display};
use util::later::Later;
use util::graph_cell::GraphRef;
use compile::{TokenValue, TokenSpan};
use ast::{SourceItem, ItemRef};
use ast::ty::Type;
use ast::errors::*;
use super::{Expression, BoxExpression};

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq)]
pub enum UnaryOperator {
  Not,
  Neg,
}

impl UnaryOperator {
  pub fn as_str(&self) -> &'static str {
    match *self {
      UnaryOperator::Not => "!",
      UnaryOperator::Neg => "-",
    }
  }
}

impl Display for UnaryOperator {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
  Mul,
  Div,
  Mod,
  Pow,
  Add,
  Sub,
  Eq,
  Ne,
  Lt,
  Le,
  Gt,
  Ge,
  And,
  Or,
}

impl BinaryOperator {
  fn as_str(&self) -> &'static str {
    match *self {
      BinaryOperator::Mul => "*",
      BinaryOperator::Div => "/",
      BinaryOperator::Mod => "%",
      BinaryOperator::Pow => "^",
      BinaryOperator::Add => "+",
      BinaryOperator::Sub => "-",
      BinaryOperator::Eq => "=",
      BinaryOperator::Ne => "!=",
      BinaryOperator::Lt => "<",
      BinaryOperator::Le => "<=",
      BinaryOperator::Gt => ">",
      BinaryOperator::Ge => ">=",
      BinaryOperator::And => "and",
      BinaryOperator::Or => "or",
    }
  }

  fn precedence(&self) -> u8 {
    match *self {
      | BinaryOperator::Mul
      | BinaryOperator::Div
      | BinaryOperator::Mod
      | BinaryOperator::Pow => 1,

      | BinaryOperator::Add
      | BinaryOperator::Sub => 2,

      | BinaryOperator::Eq
      | BinaryOperator::Ne
      | BinaryOperator::Lt
      | BinaryOperator::Le
      | BinaryOperator::Gt
      | BinaryOperator::Ge => 3,

      | BinaryOperator::And => 4,

      | BinaryOperator::Or => 5,
    }
  }
}

impl Display for BinaryOperator {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.as_str())
  }
}

#[derive(Debug, Serialize)]
pub struct UnaryExpr<'a> {
  operator: TokenValue<UnaryOperator>,
  subexpr: BoxExpression<'a>,
  ty: Later<ItemRef<'a, Type<'a>>>,
  span: TokenSpan,
}

impl<'a> UnaryExpr<'a> {
  pub fn new(
    operator: TokenValue<UnaryOperator>,
    subexpr: BoxExpression<'a>,
  ) -> Self
  {
    let span = operator.span().from_to(subexpr.span());
    UnaryExpr {
      operator,
      subexpr,
      ty: Later::new(),
      span,
    }
  }
}

impl<'a> Display for UnaryExpr<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}{}", self.operator, self.subexpr)
  }
}

impl<'a> SourceItem for UnaryExpr<'a> {
  fn span(&self) -> &TokenSpan {
    &self.span
  }

  fn resolve(&mut self) -> Result<()> {
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> Expression<'a> for UnaryExpr<'a> {
  fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.ty.item().unwrap()
  }

  fn is_constant(&self) -> bool {
    self.subexpr.is_constant()
  }
}

#[derive(Debug, Serialize)]
pub struct BinaryExpr<'a> {
  operator: TokenValue<BinaryOperator>,
  left: BoxExpression<'a>,
  right: BoxExpression<'a>,
  ty: Later<ItemRef<'a, Type<'a>>>,
  span: TokenSpan,
}

impl<'a> BinaryExpr<'a> {
  pub fn new(
    operator: TokenValue<BinaryOperator>,
    left: BoxExpression<'a>,
    right: BoxExpression<'a>,
  ) -> Self
  {
    let span = left.span().from_to(right.span());
    BinaryExpr {
      operator,
      left,
      right,
      ty: Later::new(),
      span,
    }
  }
}

impl<'a> Display for BinaryExpr<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} {} {}", self.left, self.operator, self.right)
  }
}

impl<'a> SourceItem for BinaryExpr<'a> {
  fn span(&self) -> &TokenSpan {
    &self.span
  }

  fn resolve(&mut self) -> Result<()> {
    //self.left.resolve()
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> Expression<'a> for BinaryExpr<'a> {
  fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.ty.item().unwrap()
  }

  fn is_constant(&self) -> bool {
    self.left.is_constant() && self.right.is_constant()
  }
}
