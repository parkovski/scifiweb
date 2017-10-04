use std::sync::Arc;
use std::fmt::{self, Display};
use fxhash::FxHashMap;
use util::graph_cell::GraphRef;
use util::later::Later;
use compile::{TokenSpan, TokenValue};
//use ast::var::{Scope, Variable};
//use ast::ty::{PrimitiveType, Type};
use ast::*;
use super::{Expression, ExpressionKind, BoxExpression};

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
  scope_filter: ScopeFilter<'a>,
  var: Later<GraphRef<'a, Variable<'a>>>,
}

impl<'a> ExprVar<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    scope_filter: ScopeFilter<'a>
  ) -> Self
  {
    ExprVar {
      name,
      scope_filter,
      var: Later::new(),
    }
  }
}

impl<'a> Display for ExprVar<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(&self.name)
  }
}

impl<'a> SourceItem for ExprVar<'a> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    match self.scope_filter.find(&self.name) {
      Some(v) => Ok(self.var.set(v)),
      None => Err(ErrorKind::NotDefined(self.name.clone(), "variable").into()),
    }
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> Expression<'a> for ExprVar<'a> {
  fn kind(&self) -> ExpressionKind {
    ExpressionKind::Var
  }

  fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.var.awake().ty()
  }

  fn is_constant(&self) -> bool {
    false
  }

  fn set_scope_filter(&mut self, filter: ScopeFilter<'a>) -> bool {
    self.scope_filter = filter;
    true
  }

  fn set_scope_filter_kind(&mut self, kind: ScopeKind) -> bool {
    self.scope_filter.set_kind(kind);
    true
  }
}

#[derive(Debug, Serialize)]
pub enum Literal<'a> {
  Option(TokenValue<bool>),
  Text(TokenValue<Arc<str>>),
  LocalizedText(TokenValue<Arc<str>>),
  Integer(TokenValue<i64>),
  Decimal(TokenValue<f64>),
  //DateTime(???),
  TimeSpan(Vec<TimeSpanPart>),
  Object(FxHashMap<TokenValue<Arc<str>>, BoxExpression<'a>>),
  Array(Vec<BoxExpression<'a>>),
}

impl<'a> Literal<'a> {
  pub fn primitive_type(&self) -> PrimitiveType {
    match *self {
      Literal::Option(_) => PrimitiveType::Option,
      Literal::Text(_) => PrimitiveType::Text,
      Literal::LocalizedText(_) => PrimitiveType::LocalizedText,
      Literal::Integer(_) => PrimitiveType::Integer,
      Literal::Decimal(_) => PrimitiveType::Decimal,
      Literal::TimeSpan(_) => PrimitiveType::TimeSpan,
      Literal::Object(_) => PrimitiveType::Object,
      Literal::Array(_) => PrimitiveType::Array,
    }
  }
}

impl<'a> Display for Literal<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Literal::Option(ref o) => f.write_str(if *o.value() { "yes" } else { "no" }),
      Literal::Text(ref t) => f.write_str(t.value()),
      Literal::LocalizedText(ref t) => f.write_str(t.value()),
      Literal::Integer(ref i) => write!(f, "{}", i.value()),
      Literal::Decimal(ref d) => write!(f, "{}", d.value()),
      Literal::TimeSpan(ref parts) => {
        debug_assert!(!parts.is_empty());
        for (i, part) in parts.into_iter().enumerate() {
          if i > 0 { f.write_str(" ")?; }
          write!(f, "{} {}", part.amount(), part.unit())?;
        }
        Ok(())
      }
      Literal::Object(ref _o) => unimplemented!(),
      Literal::Array(ref _a) => unimplemented!(),
    }
  }
}

#[derive(Debug, Serialize)]
pub struct ExprLiteral<'a> {
  literal: Literal<'a>,
  ty: GraphRef<'a, Type<'a>>,
}

impl<'a> ExprLiteral<'a> {
  pub fn new(literal: Literal<'a>, ty: GraphRef<'a, Type<'a>>) -> Self {
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
      Literal::Object(ref _o) => unimplemented!(),
      Literal::Array(ref _a) => unimplemented!(),
    }
  }

  fn resolve(&mut self) -> Result<()> {
    if let Literal::Object(ref mut o) = self.literal {
      for expr in o.values_mut() {
        expr.resolve()?;
      }
    } else if let Literal::Array(ref mut a) = self.literal {
      for expr in a {
        expr.resolve()?;
      }
    }
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    if let Literal::Object(ref mut o) = self.literal {
      for expr in o.values_mut() {
        expr.typecheck()?;
      }
    } else if let Literal::Array(ref mut a) = self.literal {
      for expr in a {
        expr.typecheck()?;
      }
    }
    Ok(())
  }
}

impl<'a> Expression<'a> for ExprLiteral<'a> {
  fn kind(&self) -> ExpressionKind {
    ExpressionKind::Literal
  }

  fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.ty
  }

  fn is_constant(&self) -> bool {
    match self.literal {
      Literal::Object(_) | Literal::Array(_) => false,
      _ => true,
    }
  }
}
