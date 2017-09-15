use std::sync::Arc;
use std::fmt::{self, Display};
use util::graph_cell::GraphRef;
use util::later::Later;
use compile::{TokenSpan, TokenValue};
//use ast::var::{Scope, Variable};
//use ast::ty::{PrimitiveType, Type};
use ast::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
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

#[derive(Debug, Clone)]
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

pub struct ExprLiteral<'a> {
  literal: Literal,
  ty: GraphRef<'a, Type<'a>>,
}

impl<'a> ExprLiteral<'a> {
  pub fn new(literal: Literal, ty: GraphRef<'a, Type<'a>>) -> Self {
    ExprLiteral { literal, ty }
  }
}
