use std::str;
use std::fmt::{self, Debug, Display};
use std::sync::Arc;
use std::path::PathBuf;
use std::ops::Deref;
use std::borrow::Borrow;
use std::hash::{Hash, Hasher};
use std::default::Default;
use fxhash::FxHashMap;
use serde::ser::{Serialize, Serializer, SerializeStruct};

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TokenSpan {
  pub filename: Arc<PathBuf>,
  pub line: usize,
  pub end_line: usize,
  pub start: usize,
  pub end: usize,
}

impl TokenSpan {
  pub fn new(filename: Arc<PathBuf>) -> Self {
    TokenSpan { filename, line: 1, end_line: 1, start: 1, end: 1 }
  }

  pub fn with_position(filename: Arc<PathBuf>, line: usize, start: usize, end: usize) -> Self {
    TokenSpan { filename, line, end_line: line, start, end }
  }

  pub fn from_to(&self, other: &TokenSpan) -> Self {
    if self.filename != other.filename { return self.clone(); }
    TokenSpan {
      filename: self.filename.clone(),
      line: self.line,
      end_line: other.end_line,
      start: self.start,
      end: other.end,
    }
  }
}

impl Display for TokenSpan {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}: ({}, {})", &self.filename.display(), self.line, self.start)
  }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize)]
pub struct Token<'a> {
  pub kind: TokenKind<'a>,
  pub span: TokenSpan,
}

impl<'a> Token<'a> {
  pub fn new(kind: TokenKind<'a>, span: TokenSpan) -> Self {
    Token { kind, span }
  }
}

impl<'a> fmt::Display for Token<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} at {}", &self.kind, &self.span)
  }
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize)]
pub enum TokenKind<'a> {
  Invalid(char),
  Eof,
  Identifier(&'a str),
  String(&'a str),
  Integer(i64),
  Decimal(f64),
  Percentage(f64),
  Keyword(Keyword),
  Semicolon,
  Colon,
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
  LeftArrow,
  RightArrow,
}

impl<'a> TokenKind<'a> {
  pub fn as_str(&self) -> &str {
    use self::TokenKind as TK;
    match *self {
      TK::Invalid(_) => "invalid character",
      TK::Eof => "end of input",
      TK::Identifier(s) => s,
      TK::String(s) => s,
      TK::Integer(_) => "integer",
      TK::Decimal(_) => "decimal",
      TK::Percentage(_) => "percentage",
      TK::Keyword(k) => k.as_str(),
      TK::Semicolon => ";",
      TK::Colon => ":",
      TK::Dot => ".",
      TK::Comma => ",",
      TK::LParen => "(",
      TK::RParen => ")",
      TK::LSquareBracket => "[",
      TK::RSquareBracket => "]",
      TK::Minus => "-",
      TK::Plus => "+",
      TK::Multiply => "*",
      TK::Divide => "/",
      TK::Caret => "^",
      TK::Equal => "=",
      TK::NotEqual => "!=",
      TK::Less => "<",
      TK::LessEqual => "<=",
      TK::Greater => ">",
      TK::GreaterEqual => ">=",
      TK::PercentSign => "%",
      TK::Exclamation => "!",
      TK::LeftArrow => "<-",
      TK::RightArrow => "->",
    }
  }
}

impl<'a> AsRef<str> for TokenKind<'a> {
  fn as_ref(&self) -> &str {
    self.as_str()
  }
}

impl<'a> Display for TokenKind<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    use self::TokenKind as TK;
    match *self {
      TK::Invalid(c) => write!(f, "invalid character '{}'", c),
      TK::Eof => write!(f, "end of input"),
      TK::Identifier(s) => write!(f, "identifier {}", s),
      TK::String(s) => write!(f, "string '{}'", s),
      TK::Integer(i) => write!(f, "integer {}", i),
      TK::Decimal(d) => write!(f, "decimal {}", d),
      TK::Percentage(p) => write!(f, "percentage {}%", p),
      other @ _ => f.write_str(other.as_str()),
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenValue<T>
where
  T: Debug + Display + Clone + PartialEq,
{
  value: T,
  span: TokenSpan,
}

impl<T> Eq for TokenValue<T>
where
  T: Debug + Display + Clone + PartialEq + Eq
{}

impl<T> TokenValue<T>
where
  T: Debug + Display + Clone + PartialEq,
{
  pub fn new(value: T, span: TokenSpan) -> Self {
    TokenValue { value, span }
  }

  /// Assigns a default value
  pub fn with_default(other: &TokenValue<T>) -> Self where T: Default {
    TokenValue {
      value: Default::default(),
      span: other.span.clone()
    }
  }

  pub fn with_span_range(value: T, start: &TokenSpan, end: &TokenSpan) -> Self {
    TokenValue { value, span: start.from_to(end) }
  }

  pub fn value(&self) -> &T {
    &self.value
  }

  pub fn span(&self) -> &TokenSpan {
    &self.span
  }

  pub fn into_inner(self) -> T {
    self.value
  }
}

impl<T> Display for TokenValue<T>
where
  T: Debug + Display + Clone + PartialEq,
{
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}: {}", &self.span, &self.value)
  }
}

impl<T> Deref for TokenValue<T>
where
  T: Debug + Display + Clone + PartialEq,
{
  type Target = T;
  fn deref(&self) -> &T {
    &self.value
  }
}

impl<T> Borrow<T> for TokenValue<T>
where
  T: Debug + Display + Clone + PartialEq,
{
  fn borrow(&self) -> &T {
    &self.value
  }
}

impl Borrow<str> for TokenValue<Arc<str>> {
  fn borrow(&self) -> &str {
    &self.value
  }
}

impl<T> Default for TokenValue<T>
where
  T: Default + Debug + Display + Clone + PartialEq,
{
  fn default() -> Self {
    TokenValue { value: T::default(), span: TokenSpan::default() }
  }
}

// Only hash the value so it can be used as a name key
// in a hash map.
impl<T> Hash for TokenValue<T>
where
  T: Hash + Debug + Display + Clone + PartialEq,
{
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.value.hash(state);
  }
}

impl<T> Serialize for TokenValue<T>
where
  T: Debug + Display + Clone + PartialEq + Serialize,
{
  fn serialize<S: Serializer>(&self, serializer: S)
    -> ::std::result::Result<S::Ok, S::Error>
  {
    let mut state = serializer.serialize_struct("TokenValue", 2)?;
    state.serialize_field("value", &format!("{}", self.value))?;
    state.serialize_field("span", &self.span)?;
    state.end()
  }
}

/// For tokens that have values inside them.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenMatch {
  Invalid,
  Identifier,
  String,
  Integer,
  Decimal,
  Percentage,
  Keyword,
}

impl AsRef<str> for TokenMatch {
  fn as_ref(&self) -> &str {
    match *self {
      TokenMatch::Invalid => "invalid",
      TokenMatch::Identifier => "identifier",
      TokenMatch::String => "string",
      TokenMatch::Integer => "integer",
      TokenMatch::Decimal => "decimal",
      TokenMatch::Percentage => "percentage",
      TokenMatch::Keyword => "keyword",
    }
  }
}

impl Display for TokenMatch {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(self.as_ref())
  }
}

impl<'a> PartialEq<Token<'a>> for TokenKind<'a> {
  fn eq(&self, other: &Token<'a>) -> bool {
    *self == other.kind
  }
}

impl<'a> PartialEq<TokenKind<'a>> for Token<'a> {
  fn eq(&self, other: &TokenKind<'a>) -> bool {
    *other == *self
  }
}

impl<'a> PartialEq<Token<'a>> for Keyword {
  fn eq(&self, other: &Token<'a>) -> bool {
    if let TokenKind::Keyword(k) = other.kind {
      k == *self
    } else {
      false
    }
  }
}

impl<'a> PartialEq<Keyword> for Token<'a> {
  fn eq(&self, other: &Keyword) -> bool {
    *other == *self
  }
}

impl<'a> PartialEq<Token<'a>> for TokenMatch {
  fn eq(&self, other: &Token<'a>) -> bool {
    match other.kind {
      TokenKind::Invalid(_) => *self == TokenMatch::Invalid,
      TokenKind::Identifier(_) => *self == TokenMatch::Identifier,
      TokenKind::String(_) => *self == TokenMatch::String,
      TokenKind::Integer(_) => *self == TokenMatch::Integer,
      TokenKind::Decimal(_) => *self == TokenMatch::Decimal,
      TokenKind::Percentage(_) => *self == TokenMatch::Percentage,
      TokenKind::Keyword(_) => *self == TokenMatch::Keyword,
      _ => false,
    }
  }
}

impl<'a> PartialEq<TokenMatch> for Token<'a> {
  fn eq(&self, other: &TokenMatch) -> bool {
    *other == *self
  }
}

macro_rules! keywords {
  ( $map:ident, $typ:ident, $($s:expr => $enm:ident),+ ) => (
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
    pub enum $typ {
      $($enm),+
    }

    impl $typ {
      pub fn as_str(&self) -> &'static str {
        match *self {
          $($typ::$enm => $s),+
        }
      }
    }

    impl AsRef<str> for $typ {
      fn as_ref(&self) -> &str {
        self.as_str()
      }
    }

    impl Display for $typ {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
      }
    }

    lazy_static! {
      static ref $map: FxHashMap<&'static str, $typ> = {
        let mut map = FxHashMap::default();

        $( map.insert($s, $typ::$enm) );+ ;
        map
      };
    }
  );
}

keywords! {
  KEYWORDS, Keyword,

  "option" => Option,
  "text" => Text,
  "integer" => Integer,
  "decimal" => Decimal,
  "localized" => Localized,
  "datetime" => Datetime,
  "timespan" => Timespan,

  "object" => Object,
  "array" => Array,
  "remote" => Remote,
  "user" => User,
  "group" => Group,
  "collectable" => Collectable,
  "event" => Event,
  "function" => Function,

  "yes" => Yes,
  "no" => No,
  "milliseconds" => Milliseconds,
  "seconds" => Seconds,
  "minutes" => Minutes,
  "hours" => Hours,
  "days" => Days,
  "weeks" => Weeks,
  "months" => Months,
  "years" => Years,

  "amount" => Amount,
  "cost" => Cost,
  "currency" => Currency,
  "weighted" => Weighted,
  "distribution" => Distribution,
  "range" => Range,
  "min" => Min,
  "max" => Max,
  "x" => X,

  "property" => Property,
  "permission" => Permission,
  "type" => Type,
  "tag" => Tag,
  "upgrades" => Upgrades,
  "redemptions" => Redemptions,

  "params" => Params,
  "assert" => Assert,
  "authorize" => Authorize,
  "award" => Award,
  "timer" => Timer,
  "set" => Set,
  "find" => Find,
  "notify" => Notify,
  "random" => Random,
  "if" => If,
  "else" => Else,
  "do" => Do,
  "end" => End,

  "include" => Include,
  "in" => In,
  "with" => With,
  "of" => Of,
  "to" => To,
  "for" => For,
  "similar" => Similar,
  "has" => Has,
  "and" => And,
  "or" => Or
}

pub fn get_id_or_keyword<'a>(chars: &'a [u8]) -> TokenKind<'a> {
  let id = str::from_utf8(chars).unwrap();
  let keyword = KEYWORDS.get(id);
  if let Some(&k) = keyword {
    TokenKind::Keyword(k)
  } else {
    TokenKind::Identifier(id)
  }
}
