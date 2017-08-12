use std::str;
use std::fmt;
use std::sync::Arc;
use std::path::PathBuf;
use std::ops::Add;
use fxhash::FxHashMap;
use nom::*;

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
    write!(
      f,
      "{:?} at {} ({}, {})",
      &self.kind,
      self.span.filename.display(),
      self.span.line,
      self.span.start,
    )
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

named!(
  op_semicolon<&[u8], TokenKind>,
  do_parse!(tag!(";") >> (TokenKind::Semicolon))
);
named!(
  op_dot<&[u8], TokenKind>,
  do_parse!(tag!(".") >> (TokenKind::Dot))
);
named!(
  op_comma<&[u8], TokenKind>,
  do_parse!(tag!(",") >> (TokenKind::Comma))
);
named!(
  op_lparen<&[u8], TokenKind>,
  do_parse!(tag!("(") >> (TokenKind::LParen))
);
named!(
  op_rparen<&[u8], TokenKind>,
  do_parse!(tag!(")") >> (TokenKind::RParen))
);
named!(
  op_lsquarebracket<&[u8], TokenKind>,
  do_parse!(tag!("[") >> (TokenKind::LSquareBracket))
);
named!(
  op_rsquarebracket<&[u8], TokenKind>,
  do_parse!(tag!("]") >> (TokenKind::RSquareBracket))
);
named!(
  op_minus<&[u8], TokenKind>,
  do_parse!(tag!("-") >> (TokenKind::Minus))
);
named!(
  op_plus<&[u8], TokenKind>,
  do_parse!(tag!("+") >> (TokenKind::Plus))
);
named!(
  op_multiply<&[u8], TokenKind>,
  do_parse!(tag!("*") >> (TokenKind::Multiply))
);
named!(
  op_divide<&[u8], TokenKind>,
  do_parse!(tag!("/") >> (TokenKind::Divide))
);
named!(
  op_caret<&[u8], TokenKind>,
  do_parse!(tag!("^") >> (TokenKind::Caret))
);
named!(
  op_equal<&[u8], TokenKind>,
  do_parse!(tag!("=") >> (TokenKind::Equal))
);
named!(
  op_notequal<&[u8], TokenKind>,
  do_parse!(tag!("!=") >> (TokenKind::NotEqual))
);
named!(
  op_less<&[u8], TokenKind>,
  do_parse!(tag!("<") >> (TokenKind::Less))
);
named!(
  op_greater<&[u8], TokenKind>,
  do_parse!(tag!(">") >> (TokenKind::Greater))
);
named!(
  op_lessequal<&[u8], TokenKind>,
  do_parse!(tag!("<=") >> (TokenKind::LessEqual))
);
named!(
  op_greaterequal<&[u8], TokenKind>,
  do_parse!(tag!(">=") >> (TokenKind::GreaterEqual))
);
named!(
  op_percentsign<&[u8], TokenKind>,
  do_parse!(tag!("%") >> (TokenKind::PercentSign))
);
named!(
  op_exclamation<&[u8], TokenKind>,
  do_parse!(tag!("!") >> (TokenKind::Exclamation))
);

named!(
  operator<&[u8], TokenKind>,
  alt_complete!(
    op_semicolon
    | op_dot
    | op_comma
    | op_lparen
    | op_rparen
    | op_lsquarebracket
    | op_rsquarebracket
    | op_minus
    | op_plus
    | op_multiply
    | op_divide
    | op_caret
    | op_equal
    | op_notequal
    | op_less
    | op_greater
    | op_lessequal
    | op_greaterequal
    | op_percentsign
    | op_exclamation
  )
);

fn is_integer(ch: u8) -> bool {
  ch >= b'0' && ch <= b'9'
}

fn is_identifier_begin(ch: u8) -> bool {
  (ch >= b'a' && ch <= b'z')
  || (ch >= b'A' && ch <= b'Z')
  || (ch == b'_')
}

fn is_identifier_char(ch: u8) -> bool {
  is_identifier_begin(ch) || is_integer(ch)
}

fn get_id_or_keyword(first: &[u8], rest: &[u8], colon: bool) -> TokenKind {
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

named!(
  identifier<&[u8], TokenKind>,
  alt!(regular_identifier | escaped_identifier)
);

named!(
  regular_identifier<&[u8], TokenKind>,
  do_parse!(
    first: verify!(take!(1), |c: &[u8]| is_identifier_begin(c[0])) >>
    rest: take_while!(is_identifier_char) >>
    colon: opt!(complete!(tag!(":"))) >>
    (get_id_or_keyword(first, rest, colon.is_some()))
  )
);

named!(
  escaped_identifier<&[u8], TokenKind>,
  do_parse!(
    s: preceded!(tag!("`"), take_while1!(is_identifier_char)) >>
    (TokenKind::Identifier(str::from_utf8(s).unwrap().to_owned()))
  )
);

fn get_number(int: &[u8], dec: Option<&[u8]>, pct: Option<&[u8]>) -> TokenKind {
  let is_percent = pct.is_some();
  if let Some(dec) = dec {
    let mut s = str::from_utf8(int).unwrap().to_owned();
    s.push_str(str::from_utf8(dec).unwrap());
    if is_percent {
      TokenKind::Percentage(s.parse().unwrap())
    } else {
      TokenKind::Decimal(s.parse().unwrap())
    }
  } else {
    let s = str::from_utf8(int).unwrap();
    if is_percent {
      TokenKind::Percentage(s.parse().unwrap())
    } else {
      TokenKind::Integer(s.parse().unwrap())
    }
  }
}

named!(
  number<&[u8], TokenKind>,
  do_parse!(
    int: take_while1!(is_integer) >>
    dec: opt!(recognize!(preceded!(tag!("."), take_while1!(is_integer)))) >>
    pct: opt!(complete!(tag!("%"))) >>
    (get_number(int, dec, pct))
  )
);

fn string_mid<'a>(inp: &'a [u8]) -> IResult<&'a [u8], &'a [u8]> {
  let len = inp.len();
  let quote = len > 0 && inp[0] == b'\'';
  let dbl_quote = len > 1 && inp[1] == b'\'';
  if quote {
    if dbl_quote {
      IResult::Done(&inp[2..], &inp[0..2])
    } else {
      IResult::Error(ErrorKind::Custom(1000))
    }
  } else if inp[0] == b'\n' {
    IResult::Error(ErrorKind::Custom(1000))
  } else if len > 0 {
    IResult::Done(&inp[1..], &inp[0..1])
  } else {
    IResult::Error(ErrorKind::Custom(1000))
  }
}

named!(
  string<&[u8], TokenKind>,
  do_parse!(
    s: delimited!(
      tag!("'"),
      fold_many0!(
        call!(string_mid),
        String::new(),
        |mut s: String, c| { s.push_str(str::from_utf8(c).unwrap()); s }
      ),
      tag!("'")
    ) >>
    (TokenKind::String(s))
  )
);

named!(
  line_comment<&[u8], &[u8]>,
  recognize!(do_parse!(
    char!('#') >> is_not!("\n") >> ()
  ))
);

named!(
  whitespace<&[u8], (usize, usize)>,
  fold_many0!(
    alt_complete!(
      recognize!(one_of!(" \t\r\n"))
      | line_comment
    ),
    (0, 0),
    |(lines, columns): (usize, usize), item: &[u8]| {
      match item[0] {
        b' ' => (lines, columns + 1),
        b'\t' => (lines, columns + 2 /* TODO: Configurable */),
        b'\r' | b'#' => (lines, columns),
        b'\n' => (lines + 1, 0),
        _ => unreachable!(),
      }
    }
  )
);

named!(
  invalid<&[u8], TokenKind>,
  map!(take!(1), |b| TokenKind::Invalid(b[0].into()))
);

fn map<I, O, E, F>(inp: I, expr: F) -> IResult<I, O, E>
where
  I: Copy,
  F: FnOnce(I) -> O
{
  IResult::Done(inp, expr(inp))
}

/// Returns (ws lines, ws columns, token length, token kind).
named!(
  lex_one_token<&[u8], (usize, usize, usize, TokenKind)>,
  do_parse!(
    ws: whitespace >>
    start_len: call!(map, (<[u8]>::len)) >>
    kind: alt_complete!(
      operator
      | identifier
      | number
      | string
      | map!(eof!(), |_| TokenKind::Eof)
      | invalid
    ) >>
    end_len: call!(map, (<[u8]>::len)) >>
    ((ws.0, ws.1, start_len - end_len, kind))
  )
);

pub fn next_token<'a>(inp: &'a [u8], last_token_span: &TokenSpan) -> IResult<&'a [u8], Token> {
  let result = lex_one_token(inp);
  match result {
    IResult::Done(next_inp, outp) => {
      let (ws_lines, ws_cols, tok_len, tok_kind) = outp;
      let start_col = ws_cols + if ws_lines > 0 {
          1
        } else {
          last_token_span.end
        };
      let end_col = start_col + tok_len;
      let line = last_token_span.line + ws_lines;
      let span = TokenSpan::with_position(
        last_token_span.filename.clone(),
        line,
        start_col,
        end_col
      );
      IResult::Done(next_inp, Token::new(tok_kind, span))
    }
    IResult::Incomplete(i) => IResult::Incomplete(i),
    IResult::Error(e) => IResult::Error(e),
  }
}
