use std::str;
use std::fmt::{self, Debug, Display};
use std::sync::Arc;
use std::path::PathBuf;
use std::ops::{Deref, DerefMut};
use fxhash::FxHashMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TokenSpan {
  pub filename: Arc<PathBuf>,
  pub line: usize,
  pub start: usize,
  pub end: usize,
}

impl TokenSpan {
  pub fn new(filename: Arc<PathBuf>) -> Self {
    TokenSpan { filename, line: 1, start: 1, end: 1 }
  }

  pub fn with_position(filename: Arc<PathBuf>, line: usize, start: usize, end: usize) -> Self {
    TokenSpan { filename, line, start, end }
  }
}

impl Display for TokenSpan {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}: ({}, {})", &self.filename.display(), self.line, self.start)
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct Token {
  pub kind: TokenKind,
  pub span: TokenSpan,
}

impl Token {
  pub fn new(kind: TokenKind, span: TokenSpan) -> Self {
    Token { kind, span }
  }
}

impl fmt::Display for Token {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?} at {}", &self.kind, &self.span)
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct TokenValue<T: Display + Debug + Clone + PartialEq + PartialOrd> {
  pub value: T,
  pub kind: &'static str,
  pub span: TokenSpan,
}

impl<T: Display + Debug + Clone + PartialEq + PartialOrd> TokenValue<T> {
  pub fn new(value: T, kind: &'static str, span: TokenSpan) -> Self {
    TokenValue { value, kind, span }
  }
}

impl<T: Display + Debug + Clone + PartialEq + PartialOrd> TokenValue<T> {
  pub fn to_string_value(&self) -> TokenValue<String> {
    TokenValue {
      value: self.value.to_string(),
      kind: self.kind,
      span: self.span.clone(),
    }
  }

  pub fn into_string_value(self) -> TokenValue<String> {
    TokenValue {
      value: self.value.to_string(),
      kind: self.kind,
      span: self.span,
    }
  }

  pub fn into_inner(self) -> T {
    self.value
  }
}

impl<T: Display + Debug + Clone + PartialEq + PartialOrd> Deref for TokenValue<T> {
  type Target = T;
  fn deref(&self) -> &T {
    &self.value
  }
}

impl<T: Display + Debug + Clone + PartialEq + PartialOrd> DerefMut for TokenValue<T> {
  fn deref_mut(&mut self) -> &mut T {
    &mut self.value
  }
}

impl<T: Display + Debug + Clone + PartialEq + PartialOrd> fmt::Display for TokenValue<T> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} at {}", &self.kind, &self.span)
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum TokenKind {
  Invalid(char),
  Eof,
  Identifier(String),
  Label(String),
  String(String),
  Integer(i64),
  Decimal(f64),
  Percentage(f64),
  Keyword(Keyword),
  Semicolon,
  Dot,
  Comma,
  LParen,
  RParen,
  LSquareBracket,
  RSquareBracket,
  Minus,
  Plus,
  Multiply,
  Divide,
  Caret,
  Equal,
  NotEqual,
  Less,
  LessEqual,
  Greater,
  GreaterEqual,
  PercentSign,
  Exclamation,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Keyword {
  // Types
  Switch,
  Text,
  Localized,
  Gameserver,
  Admin,
  Datetime,
  Gameresult,

  // Definition types
  Random,
  User,
  Group,
  Collectable,
  Event,
  Map,

  // Special type values
  On,
  Off,
  Seconds,
  Minutes,
  Hours,
  Days,
  Weeks,
  Months,

  // Numerical
  Amount,
  Cost,
  Currency,
  Weighted,
  Distribution,
  Range,
  Min,
  Max,
  X,

  // Object definitions
  Property,
  Permission,
  Type,
  Tag,
  Upgrades,
  Redemptions,

  // Events
  Params,
  Assert,
  Authorize,
  Award,
  Timer,
  Set,
  Find,
  Notify,
  Option,

  // Random grammar
  Include,
  In,
  With,
  Of,
  To,
  For,
  Similar,
  Has,
  And,
  Or
}

lazy_static! {
  static ref KEYWORDS: FxHashMap<&'static str, Keyword> = {
    let mut map = FxHashMap::default();
    
    map.insert("switch", Keyword::Switch);
    map.insert("text", Keyword::Text);
    map.insert("localized", Keyword::Localized);
    map.insert("gameserver", Keyword::Gameserver);
    map.insert("admin", Keyword::Admin);
    map.insert("datetime", Keyword::Datetime);
    map.insert("gameresult", Keyword::Gameresult);

    map.insert("random", Keyword::Random);
    map.insert("user", Keyword::User);
    map.insert("group", Keyword::Group);
    map.insert("collectable", Keyword::Collectable);
    map.insert("event", Keyword::Event);
    map.insert("map", Keyword::Map);

    map.insert("on", Keyword::On);
    map.insert("off", Keyword::Off);
    map.insert("seconds", Keyword::Seconds);
    map.insert("minutes", Keyword::Minutes);
    map.insert("hours", Keyword::Hours);
    map.insert("days", Keyword::Days);
    map.insert("weeks", Keyword::Weeks);
    map.insert("months", Keyword::Months);

    map.insert("amount", Keyword::Amount);
    map.insert("cost", Keyword::Cost);
    map.insert("currency", Keyword::Currency);
    map.insert("weighted", Keyword::Weighted);
    map.insert("distribution", Keyword::Distribution);
    map.insert("range", Keyword::Range);
    map.insert("min", Keyword::Min);
    map.insert("max", Keyword::Max);
    map.insert("x", Keyword::X);

    map.insert("property", Keyword::Property);
    map.insert("permission", Keyword::Permission);
    map.insert("type", Keyword::Type);
    map.insert("tag", Keyword::Tag);
    map.insert("upgrades", Keyword::Upgrades);
    map.insert("redemptions", Keyword::Redemptions);
    
    map.insert("params", Keyword::Params);
    map.insert("assert", Keyword::Assert);
    map.insert("authorize", Keyword::Authorize);
    map.insert("award", Keyword::Award);
    map.insert("timer", Keyword::Timer);
    map.insert("set", Keyword::Set);
    map.insert("find", Keyword::Find);
    map.insert("notify", Keyword::Notify);
    map.insert("option", Keyword::Option);

    map.insert("include", Keyword::Include);
    map.insert("in", Keyword::In);
    map.insert("with", Keyword::With);
    map.insert("of", Keyword::Of);
    map.insert("to", Keyword::To);
    map.insert("for", Keyword::For);
    map.insert("similar", Keyword::Similar);
    map.insert("has", Keyword::Has);
    map.insert("and", Keyword::And);
    map.insert("or", Keyword::Or);

    map
  };
}

pub fn get_id_or_keyword(first: &[u8], rest: &[u8], colon: bool) -> TokenKind {
  let mut id = str::from_utf8(first).unwrap().to_owned();
  id.push_str(str::from_utf8(rest).unwrap());
  if colon {
    return TokenKind::Label(id);
  }
  let keyword = KEYWORDS.get(id.as_str());
  if let Some(&k) = keyword {
    TokenKind::Keyword(k)
  } else {
    TokenKind::Identifier(id)
  }
}
