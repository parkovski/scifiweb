use std::str;
use std::fmt::{self, Debug, Display};
use std::sync::Arc;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::ops::Deref;
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

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum TokenKind<'a> {
  Invalid(char),
  Eof,
  Identifier(&'a str),
  Label(&'a str),
  String(&'a str),
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

impl<'a> Display for TokenKind<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    use self::TokenKind as TK;
    match *self {
      TK::Invalid(c) => write!(f, "invalid character '{}'", c),
      TK::Eof => write!(f, "end of input"),
      TK::Identifier(s) => write!(f, "identifier {}", s),
      TK::Label(s) => write!(f, "label {}:", s),
      TK::String(s) => write!(f, "string '{}'", s),
      TK::Integer(i) => write!(f, "integer {}", i),
      TK::Decimal(d) => write!(f, "decimal {}", d),
      TK::Percentage(p) => write!(f, "percentage {}%", p),
      TK::Keyword(k) => write!(f, "{}", k),
      TK::Semicolon => write!(f, ";"),
      TK::Dot => write!(f, "."),
      TK::Comma => write!(f, ","),
      TK::LParen => write!(f, "("),
      TK::RParen => write!(f, ")"),
      TK::LSquareBracket => write!(f, "["),
      TK::RSquareBracket => write!(f, "]"),
      TK::Minus => write!(f, "-"),
      TK::Plus => write!(f, "+"),
      TK::Multiply => write!(f, "*"),
      TK::Divide => write!(f, "/"),
      TK::Caret => write!(f, "^"),
      TK::Equal => write!(f, "="),
      TK::NotEqual => write!(f, "!="),
      TK::Less => write!(f, "<"),
      TK::LessEqual => write!(f, "<="),
      TK::Greater => write!(f, ">"),
      TK::GreaterEqual => write!(f, ">="),
      TK::PercentSign => write!(f, "%"),
      TK::Exclamation => write!(f, "!"),
    }
  }
}

impl<'a> From<Token<'a>> for TokenValue<String> {
  fn from(token: Token<'a>) -> Self {
    use self::TokenKind as TK;
    let s = match token.kind {
      TK::Invalid(c) => "invalid".to_string(),
      TK::Eof => "end of input".to_string(),
      TK::Identifier(s) => s.to_string(),
      TK::Label(s) => s.to_string(),
      TK::String(s) => s.to_string(),
      TK::Integer(i) => i.to_string(),
      TK::Decimal(d) => d.to_string(),
      TK::Percentage(p) => p.to_string(),
      TK::Keyword(k) => k.to_string(),
      other @ _ => other.to_string(),
    };

    TokenValue { value: s, span: token.span }
  }
}

impl<'a> From<Token<'a>> for TokenValue<i64> {
  fn from(token: Token<'a>) -> Self {
    let i = if let TokenKind::Integer(i) = token.kind {
      i
    } else {
      panic!("Can't do this conversion");
    };

    TokenValue { value: i, span: token.span }
  }
}

impl<'a> From<Token<'a>> for TokenValue<f64> {
  fn from(token: Token<'a>) -> Self {
    let f = match token.kind {
      TokenKind::Decimal(d) => d,
      TokenKind::Percentage(p) => p,
      _ => panic!("Can't do this conversion"),
    };

    TokenValue { value: f, span: token.span }
  }
}

#[derive(Debug, Clone)]
pub struct TokenValue<T: Debug + Clone> {
  value: T,
  span: TokenSpan,
}

impl<T: Debug + Clone> Deref for TokenValue<T> {
  type Target = T;
  fn deref(&self) -> &T {
    &self.value
  }
}

impl<T: Debug + Clone> PartialEq for TokenValue<T> {
  fn eq(&self, other: &TokenValue<T>) -> bool {
    self.span == other.span
  }
}

impl<T: Debug + Clone> Eq for TokenValue<T> {}

impl<T: Debug + Clone> PartialOrd for TokenValue<T> {
  fn partial_cmp(&self, other: &TokenValue<T>) -> Option<Ordering> {
    self.span.partial_cmp(&other.span)
  }
}

impl<T: Debug + Clone> Ord for TokenValue<T> {
  fn cmp(&self, other: &TokenValue<T>) -> Ordering {
    self.span.cmp(&other.span)
  }
}

/// For tokens that have values inside them.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TokenMatch {
  Invalid,
  Identifier,
  Label,
  String,
  Integer,
  Decimal,
  Percentage,
  Keyword,
}

impl Display for TokenMatch {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match *self {
      TokenMatch::Invalid => "invalid",
      TokenMatch::Identifier => "identifier",
      TokenMatch::Label => "label",
      TokenMatch::String => "string",
      TokenMatch::Integer => "integer",
      TokenMatch::Decimal => "decimal",
      TokenMatch::Percentage => "percentage",
      TokenMatch::Keyword => "keyword",
    })
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
      TokenKind::Label(_) => *self == TokenMatch::Label,
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
    #[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
    pub enum $typ {
      $($enm),+
    }

    impl Display for $typ {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
          $($typ::$enm => $s),+
        })
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

  "switch" => Switch,
  "text" => Text,
  "localized" => Localized,
  "gameserver" => Gameserver,
  "admin" => Admin,
  "datetime" => Datetime,
  "gameresult" => Gameresult,

  "random" => Random,
  "user" => User,
  "group" => Group,
  "collectable" => Collectable,
  "event" => Event,
  "map" => Map,

  "on" => On,
  "off" => Off,
  "seconds" => Seconds,
  "minutes" => Minutes,
  "hours" => Hours,
  "days" => Days,
  "weeks" => Weeks,
  "months" => Months,

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
  "option" => Option,

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

pub fn get_id_or_keyword<'a>(chars: &'a [u8], colon: bool) -> TokenKind<'a> {
  let id = str::from_utf8(chars).unwrap();
  if colon {
    return TokenKind::Label(id);
  }
  let keyword = KEYWORDS.get(id);
  if let Some(&k) = keyword {
    TokenKind::Keyword(k)
  } else {
    TokenKind::Identifier(id)
  }
}
