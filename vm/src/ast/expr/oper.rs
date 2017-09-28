use std::fmt::{self, Display};
use util::later::Later;
use util::graph_cell::GraphRef;
use compile::{TokenValue, TokenSpan};
use ast::{SourceItem, ItemRef};
use ast::ty::Type;
use ast::var::{ScopeFilter, Scoped};
use ast::errors::*;
use super::{Expression, BoxExpression};

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq)]
pub enum PrefixOperator {
  Parens,
  Not,
  Neg,
  Dot,
}

impl PrefixOperator {
  pub fn str_before(&self) -> &'static str {
    match *self {
      PrefixOperator::Parens => "(",
      PrefixOperator::Not => "!",
      PrefixOperator::Neg => "-",
      PrefixOperator::Dot => ".",
    }
  }

  pub fn str_after(&self) -> Option<&'static str> {
    if *self == PrefixOperator::Parens {
      Some(")")
    } else {
      None
    }
  }

  pub fn precedence(&self) -> u8 {
    match *self {
      PrefixOperator::Parens => 0,
      PrefixOperator::Dot => 8,
      _ => 6,
    }
  }
}

impl Display for PrefixOperator {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.str_before())?;
    if let Some(after) = self.str_after() {
      f.write_str(after)?;
    }
    Ok(())
  }
}

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq)]
pub enum BinaryOperator {
  Dot,
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
  pub fn as_str(&self) -> &'static str {
    match *self {
      BinaryOperator::Dot => ".",
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

  pub fn precedence(&self) -> u8 {
    match *self {
      | BinaryOperator::Dot => 7,

      | BinaryOperator::Mul
      | BinaryOperator::Div
      | BinaryOperator::Mod
      | BinaryOperator::Pow => 5,

      | BinaryOperator::Add
      | BinaryOperator::Sub => 4,

      | BinaryOperator::Eq
      | BinaryOperator::Ne
      | BinaryOperator::Lt
      | BinaryOperator::Le
      | BinaryOperator::Gt
      | BinaryOperator::Ge => 3,

      | BinaryOperator::And => 2,

      | BinaryOperator::Or => 1,
    }
  }

  pub fn right_recursive(&self) -> bool {
    *self == BinaryOperator::Pow
  }
}

impl Display for BinaryOperator {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq)]
pub enum PostfixListOperator {
  Call,
  Idx,
}

impl PostfixListOperator {
  pub const PRECEDENCE: u8 = 7;

  pub fn str_before(&self) -> &'static str {
    match *self {
      PostfixListOperator::Call => "(",
      PostfixListOperator::Idx => "[",
    }
  }

  pub fn str_after(&self) -> &'static str {
    match *self {
      PostfixListOperator::Call => ")",
      PostfixListOperator::Idx => "]",
    }
  }
}

impl Display for PostfixListOperator {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}{}", self.str_before(), self.str_after())
  }
}

#[derive(Debug, Serialize)]
pub struct PrefixExpr<'a> {
  operator: TokenValue<PrefixOperator>,
  subexpr: BoxExpression<'a>,
  ty: Later<ItemRef<'a, Type<'a>>>,
  span: TokenSpan,
}

impl<'a> PrefixExpr<'a> {
  pub fn new(
    operator: TokenValue<PrefixOperator>,
    subexpr: BoxExpression<'a>,
  ) -> Self
  {
    let span = operator.span().from_to(subexpr.span());
    PrefixExpr {
      operator,
      subexpr,
      ty: Later::new(),
      span,
    }
  }
}

impl<'a> Display for PrefixExpr<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "{}{}",
      self.operator.value().str_before(),
      self.subexpr,
    )?;
    if let Some(after) = self.operator.value().str_after() {
      f.write_str(after)?;
    }
    Ok(())
  }
}

impl<'a> SourceItem for PrefixExpr<'a> {
  fn span(&self) -> &TokenSpan {
    &self.span
  }

  fn resolve(&mut self) -> Result<()> {
    self.subexpr.resolve()
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> Expression<'a> for PrefixExpr<'a> {
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
    write!(f, "{} {} {}", self.left, self.operator.value().as_str(), self.right)
  }
}

impl<'a> SourceItem for BinaryExpr<'a> {
  fn span(&self) -> &TokenSpan {
    &self.span
  }

  fn resolve(&mut self) -> Result<()> {
    self.left.resolve()?;
    // Special case: the '.' operator makes the right side look at the scope
    // of the left.
    if *self.operator.value() == BinaryOperator::Dot {
      let ty = self.left.ty();
      let scope = ty.awake().scope();
      let range = scope.awake().kind().only();
      let filter = ScopeFilter::new(scope, range, true);
      if !self.right.set_scope_filter(filter) {
        return Err(ErrorKind::InvalidExpression(
          self.right.to_string(),
          self.right.span().clone(),
        ).into());
      }
    }
    self.right.resolve()
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

#[derive(Debug, Serialize)]
pub struct PostfixListExpr<'a> {
  operator: TokenValue<PostfixListOperator>,
  left: BoxExpression<'a>,
  right: Vec<BoxExpression<'a>>,
  ty: Later<ItemRef<'a, Type<'a>>>,
  span: TokenSpan,
}

impl<'a> PostfixListExpr<'a> {
  pub fn new(
    operator: TokenValue<PostfixListOperator>,
    left: BoxExpression<'a>,
    right: Vec<BoxExpression<'a>>,
  ) -> Self
  {
    let span = left.span().from_to(operator.span());
    PostfixListExpr {
      operator,
      left,
      right,
      ty: Later::new(),
      span,
    }
  }
}

impl<'a> Display for PostfixListExpr<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "{}{}{:?}{}",
      self.left,
      self.operator.value().str_before(),
      self.right,
      self.operator.value().str_after(),
    )
  }
}

impl<'a> SourceItem for PostfixListExpr<'a> {
  fn span(&self) -> &TokenSpan {
    &self.span
  }

  fn resolve(&mut self) -> Result<()> {
    self.left.resolve()?;
    for e in &mut self.right {
      e.resolve()?;
    }
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> Expression<'a> for PostfixListExpr<'a> {
  fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.ty.item().unwrap()
  }

  fn is_constant(&self) -> bool {
    false
  }
}
