use std::str;
use fxhash::FxHashMap;
use nom::*;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum Token {
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
  op_semicolon<&[u8], Token>,
  do_parse!(tag!(";") >> (Token::Semicolon))
);
named!(
  op_dot<&[u8], Token>,
  do_parse!(tag!(".") >> (Token::Dot))
);
named!(
  op_comma<&[u8], Token>,
  do_parse!(tag!(",") >> (Token::Comma))
);
named!(
  op_lparen<&[u8], Token>,
  do_parse!(tag!("(") >> (Token::LParen))
);
named!(
  op_rparen<&[u8], Token>,
  do_parse!(tag!(")") >> (Token::RParen))
);
named!(
  op_lsquarebracket<&[u8], Token>,
  do_parse!(tag!("[") >> (Token::LSquareBracket))
);
named!(
  op_rsquarebracket<&[u8], Token>,
  do_parse!(tag!("]") >> (Token::RSquareBracket))
);
named!(
  op_minus<&[u8], Token>,
  do_parse!(tag!("-") >> (Token::Minus))
);
named!(
  op_plus<&[u8], Token>,
  do_parse!(tag!("+") >> (Token::Plus))
);
named!(
  op_multiply<&[u8], Token>,
  do_parse!(tag!("*") >> (Token::Multiply))
);
named!(
  op_divide<&[u8], Token>,
  do_parse!(tag!("/") >> (Token::Divide))
);
named!(
  op_caret<&[u8], Token>,
  do_parse!(tag!("^") >> (Token::Caret))
);
named!(
  op_equal<&[u8], Token>,
  do_parse!(tag!("=") >> (Token::Equal))
);
named!(
  op_notequal<&[u8], Token>,
  do_parse!(tag!("!=") >> (Token::NotEqual))
);
named!(
  op_less<&[u8], Token>,
  do_parse!(tag!("<") >> (Token::Less))
);
named!(
  op_greater<&[u8], Token>,
  do_parse!(tag!(">") >> (Token::Greater))
);
named!(
  op_lessequal<&[u8], Token>,
  do_parse!(tag!("<=") >> (Token::LessEqual))
);
named!(
  op_greaterequal<&[u8], Token>,
  do_parse!(tag!(">=") >> (Token::GreaterEqual))
);
named!(
  op_percentsign<&[u8], Token>,
  do_parse!(tag!("%") >> (Token::PercentSign))
);
named!(
  op_exclamation<&[u8], Token>,
  do_parse!(tag!("!") >> (Token::Exclamation))
);

named!(
  operator<&[u8], Token>,
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

fn get_id_or_keyword(first: &[u8], rest: &[u8], colon: bool) -> Token {
  let mut id = str::from_utf8(first).unwrap().to_owned();
  id.push_str(str::from_utf8(rest).unwrap());
  if colon {
    return Token::Label(id);
  }
  let keyword = KEYWORDS.get(id.as_str());
  if let Some(&k) = keyword {
    Token::Keyword(k)
  } else {
    Token::Identifier(id)
  }
}

named!(
  identifier<&[u8], Token>,
  alt!(regular_identifier | escaped_identifier)
);

named!(
  regular_identifier<&[u8], Token>,
  do_parse!(
    first: verify!(take!(1), |c: &[u8]| is_identifier_begin(c[0])) >>
    rest: take_while!(is_identifier_char) >>
    colon: opt!(complete!(tag!(":"))) >>
    (get_id_or_keyword(first, rest, colon.is_some()))
  )
);

named!(
  escaped_identifier<&[u8], Token>,
  do_parse!(
    s: preceded!(tag!("`"), take_while1!(is_identifier_char)) >>
    (Token::Identifier(str::from_utf8(s).unwrap().to_owned()))
  )
);

fn get_number(int: &[u8], dec: Option<&[u8]>, pct: Option<&[u8]>) -> Token {
  let is_percent = pct.is_some();
  if let Some(dec) = dec {
    let mut s = str::from_utf8(int).unwrap().to_owned();
    s.push_str(str::from_utf8(dec).unwrap());
    if is_percent {
      Token::Percentage(s.parse().unwrap())
    } else {
      Token::Decimal(s.parse().unwrap())
    }
  } else {
    let s = str::from_utf8(int).unwrap();
    if is_percent {
      Token::Percentage(s.parse().unwrap())
    } else {
      Token::Integer(s.parse().unwrap())
    }
  }
}

named!(
  number<&[u8], Token>,
  do_parse!(
    int: take_while1!(is_integer) >>
    dec: opt!(recognize!(preceded!(tag!("."), take_while1!(is_integer)))) >>
    pct: opt!(complete!(tag!("%"))) >>
    (get_number(int, dec, pct))
  )
);

named!(
  string<&[u8], Token>,
  do_parse!(
    s: delimited!(
      tag!("'"),
      fold_many0!(
        alt_complete!(tag!("''") | is_not!("'")),
        String::new(),
        |mut s: String, c| { s.push_str(str::from_utf8(c).unwrap()); s }
      ),
      tag!("'")
    ) >>
    (Token::String(s))
  )
);

named!(
  lex_bytes<&[u8], Vec<Token> >,
  ws!(many0!(alt_complete!(
    operator
    | identifier
    | number
    | string
  )))
);

pub fn lex(program: &str) -> IResult<&[u8], Vec<Token>> {
  lex_bytes(program.as_bytes())
}
