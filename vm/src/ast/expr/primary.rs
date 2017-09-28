use std::sync::Arc;
use std::fmt::{self, Display};
use util::graph_cell::GraphRef;
use util::later::Later;
use compile::{TokenSpan, TokenValue};
//use ast::var::{Scope, Variable};
//use ast::ty::{PrimitiveType, Type};
use ast::*;
use super::Expression;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[repr(u16)]
pub enum TimeSpanUnit {
  Milliseconds,
  Seconds,
  Minutes,
  Hours,
  Days,
  Weeks,
  Months,
  Years,
}

impl TimeSpanUnit {
  pub fn max_value(&self) -> i16 {
    match *self {
      // 2 seconds
      TimeSpanUnit::Milliseconds => 2000,
      // 2 minutes
      TimeSpanUnit::Seconds => 120,
      // 2 hours
      TimeSpanUnit::Minutes => 120,
      // 3 days
      TimeSpanUnit::Hours => 72,
      // 1 year
      TimeSpanUnit::Days => 365,
      // 1 year
      TimeSpanUnit::Weeks => 52,
      // 1 year
      TimeSpanUnit::Months => 12,
      // I feel like setting this to 1 would be
      // a little restrictive, but seriously,
      // who's even going to use this unit?
      TimeSpanUnit::Years => 5,
    }
  }
}

impl Display for TimeSpanUnit {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match *self {
      TimeSpanUnit::Milliseconds => "milliseconds",
      TimeSpanUnit::Seconds => "seconds",
      TimeSpanUnit::Minutes => "minutes",
      TimeSpanUnit::Hours => "hours",
      TimeSpanUnit::Days => "days",
      TimeSpanUnit::Weeks => "weeks",
      TimeSpanUnit::Months => "months",
      TimeSpanUnit::Years => "years",
    })
  }
}

#[derive(Debug, Clone, Serialize)]
pub struct TimeSpanPart {
  amount: i16,
  unit: TimeSpanUnit,
  span: TokenSpan,
}

impl TimeSpanPart {
  pub fn new(amount: TokenValue<i64>, unit: TokenValue<TimeSpanUnit>) -> Result<Self> {
    let amount_val = *amount.value();
    let unit_val = *unit.value();
    let max = unit_val.max_value() as i64;
    if amount_val > max || amount_val < -max {
      return Err(ErrorKind::ValueOutOfRange(
        amount.to_string(),
        if unit_val == TimeSpanUnit::Years {
          "let's be realistic here"
        } else {
          "use the next unit up for larger time span values \
          (e.g. '300 minutes' becomes '3 hours 30 minutes')"
        },
        amount.span().clone(),
      ).into());
    }
    Ok(TimeSpanPart {
      amount: amount_val as i16,
      unit: *unit.value(),
      span: amount.span().from_to(unit.span()),
    })
  }

  pub fn amount(&self) -> i16 {
    self.amount
  }

  pub fn unit(&self) -> TimeSpanUnit {
    self.unit
  }

  pub fn span(&self) -> &TokenSpan {
    &self.span
  }
}

#[derive(Debug, Serialize)]
pub struct ExprVar<'a> {
  name: TokenValue<Arc<str>>,
  scope: GraphRef<'a, Scope<'a>>,
  var: Later<GraphRef<'a, Variable<'a>>>,
}

impl<'a> ExprVar<'a> {
  pub fn new(name: TokenValue<Arc<str>>, scope: GraphRef<'a, Scope<'a>>) -> Self {
    ExprVar {
      name,
      scope,
      var: Later::new(),
    }
  }
}

impl<'a> Display for ExprVar<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    Display::fmt(&self.var.awake(), f)
  }
}

impl<'a> SourceItem for ExprVar<'a> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    match self.scope.awake().find(&self.name) {
      Some(v) => Ok(self.var.set(v)),
      None => Err(ErrorKind::NotDefined(self.name.clone(), "variable").into()),
    }
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> Expression<'a> for ExprVar<'a> {
  fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.var.awake().ty()
  }

  fn is_constant(&self) -> bool {
    false
  }
}

#[derive(Debug, Serialize)]
pub enum Literal {
  Option(TokenValue<bool>),
  Text(TokenValue<Arc<str>>),
  LocalizedText(TokenValue<Arc<str>>),
  Integer(TokenValue<i64>),
  Decimal(TokenValue<f64>),
  //DateTime(???),
  TimeSpan(Vec<TimeSpanPart>),
  //Object(???),
  //Array(???),
}

impl Literal {
  pub fn primitive_type(&self) -> PrimitiveType {
    match *self {
      Literal::Option(_) => PrimitiveType::Option,
      Literal::Text(_) => PrimitiveType::Text,
      Literal::LocalizedText(_) => PrimitiveType::LocalizedText,
      Literal::Integer(_) => PrimitiveType::Integer,
      Literal::Decimal(_) => PrimitiveType::Decimal,
      Literal::TimeSpan(_) => PrimitiveType::TimeSpan,
    }
  }
}

impl Display for Literal {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Literal::Option(ref o) => f.write_str(if *o.value() { "yes" } else { "no" }),
      Literal::Text(ref t) => f.write_str(&t),
      Literal::LocalizedText(ref t) => f.write_str(&t),
      Literal::Integer(ref i) => write!(f, "{}", i),
      Literal::Decimal(ref d) => write!(f, "{}", d),
      // FIXME!
      Literal::TimeSpan(ref ts) => f.write_str("timespan"),
    }
  }
}

#[derive(Debug, Serialize)]
pub struct ExprLiteral<'a> {
  literal: Literal,
  ty: GraphRef<'a, Type<'a>>,
}

impl<'a> ExprLiteral<'a> {
  pub fn new(literal: Literal, ty: GraphRef<'a, Type<'a>>) -> Self {
    ExprLiteral { literal, ty }
  }
}

impl<'a> Display for ExprLiteral<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    Display::fmt(&self.literal, f)
  }
}

impl<'a> SourceItem for ExprLiteral<'a> {
  fn span(&self) -> &TokenSpan {
    match self.literal {
      Literal::Option(ref o) => o.span(),
      Literal::Text(ref t) => t.span(),
      Literal::LocalizedText(ref t) => t.span(),
      Literal::Integer(ref i) => i.span(),
      Literal::Decimal(ref d) => d.span(),
      // FIXME!
      Literal::TimeSpan(ref ts) => ts[0].span(),
    }
  }

  fn resolve(&mut self) -> Result<()> {
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> Expression<'a> for ExprLiteral<'a> {
  fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.ty
  }

  fn is_constant(&self) -> bool {
    true
  }
}
