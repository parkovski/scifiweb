use std::str;
use fxhash::FxHashMap;
use nom::{self, IResult};
use super::token::*;
use super::parse_errors::*;

macro_rules! lexfn {
  ($name:ident -> $ty:ty, $submac:ident!( $($args:tt)* )) => (
    named!($name<&[u8], $ty, Error>, $submac)
  )
}

lexfn!(
  op_semicolon -> TokenKind,
  do_parse!(tag!(";") >> (TokenKind::Semicolon))
);
lexfn!(
  op_dot -> TokenKind,
  do_parse!(tag!(".") >> (TokenKind::Dot))
);
lexfn!(
  op_comma -> TokenKind,
  do_parse!(tag!(",") >> (TokenKind::Comma))
);
lexfn!(
  op_lparen -> TokenKind,
  do_parse!(tag!("(") >> (TokenKind::LParen))
);
lexfn!(
  op_rparen -> TokenKind,
  do_parse!(tag!(")") >> (TokenKind::RParen))
);
lexfn!(
  op_lsquarebracket -> TokenKind,
  do_parse!(tag!("[") >> (TokenKind::LSquareBracket))
);
lexfn!(
  op_rsquarebracket -> TokenKind,
  do_parse!(tag!("]") >> (TokenKind::RSquareBracket))
);
lexfn!(
  op_minus -> TokenKind,
  do_parse!(tag!("-") >> (TokenKind::Minus))
);
lexfn!(
  op_plus -> TokenKind,
  do_parse!(tag!("+") >> (TokenKind::Plus))
);
lexfn!(
  op_multiply -> TokenKind,
  do_parse!(tag!("*") >> (TokenKind::Multiply))
);
lexfn!(
  op_divide -> TokenKind,
  do_parse!(tag!("/") >> (TokenKind::Divide))
);
lexfn!(
  op_caret -> TokenKind,
  do_parse!(tag!("^") >> (TokenKind::Caret))
);
lexfn!(
  op_equal -> TokenKind,
  do_parse!(tag!("=") >> (TokenKind::Equal))
);
lexfn!(
  op_notequal -> TokenKind,
  do_parse!(tag!("!=") >> (TokenKind::NotEqual))
);
lexfn!(
  op_less -> TokenKind,
  do_parse!(tag!("<") >> (TokenKind::Less))
);
lexfn!(
  op_greater -> TokenKind,
  do_parse!(tag!(">") >> (TokenKind::Greater))
);
lexfn!(
  op_lessequal -> TokenKind,
  do_parse!(tag!("<=") >> (TokenKind::LessEqual))
);
lexfn!(
  op_greaterequal -> TokenKind,
  do_parse!(tag!(">=") >> (TokenKind::GreaterEqual))
);
lexfn!(
  op_percentsign -> TokenKind,
  do_parse!(tag!("%") >> (TokenKind::PercentSign))
);
lexfn!(
  op_exclamation -> TokenKind,
  do_parse!(tag!("!") >> (TokenKind::Exclamation))
);

lexfn!(
  operator -> TokenKind,
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

lexfn!(
  identifier -> TokenKind,
  alt!(regular_identifier | escaped_identifier)
);

lexfn!(
  regular_identifier -> TokenKind,
  do_parse!(
    first: verify!(take!(1), |c: &[u8]| is_identifier_begin(c[0])) >>
    rest: take_while!(is_identifier_char) >>
    colon: opt!(complete!(tag!(":"))) >>
    (get_id_or_keyword(first, rest, colon.is_some()))
  )
);

lexfn!(
  escaped_identifier -> TokenKind,
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

lexfn!(
  number -> TokenKind,
  do_parse!(
    int: take_while1!(is_integer) >>
    dec: opt!(recognize!(preceded!(tag!("."), take_while1!(is_integer)))) >>
    pct: opt!(complete!(tag!("%"))) >>
    (get_number(int, dec, pct))
  )
);

fn string_mid<'a>(inp: &'a [u8]) -> IResult<&'a [u8], &'a [u8], Error> {
  let len = inp.len();
  let quote = len > 0 && inp[0] == b'\'';
  let dbl_quote = len > 1 && inp[1] == b'\'';
  if quote {
    if dbl_quote {
      IResult::Done(&inp[2..], &inp[0..2])
    } else {
      unreachable!("delimited! failed")
    }
  } else if inp[0] == b'\n' {
    ErrorKind::UnclosedString(Placeholder::new()).into_nom()
  } else if len > 0 {
    IResult::Done(&inp[1..], &inp[0..1])
  } else {
    ErrorKind::UnclosedString(Placeholder::new()).into_nom()
  }
}

lexfn!(
  string -> TokenKind,
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

lexfn!(
  line_comment -> &[u8],
  recognize!(do_parse!(
    char!('#') >> is_not!("\n") >> ()
  ))
);

lexfn!(
  whitespace -> (usize, usize),
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

lexfn!(
  invalid -> TokenKind,
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
lexfn!(
  lex_one_token -> (usize, usize, usize, TokenKind),
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

pub fn next_token<'a>(inp: &'a [u8], last_token_span: &TokenSpan)
  -> IResult<&'a [u8], Token, Error>
{
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
    IResult::Error(e) => {
      if let nom::ErrorKind::Custom(ErrorKind::UnclosedString(ref mut placeholder)) = e {
        // TODO: How to get the actual span for this token?
        placeholder.fill(TokenSpan::with_position(
          last_token_span.filename.clone(),
          last_token_span.line,
          last_token_span.end_col,
          last_token_span.end_col,
        ));
      }
      IResult::Error(e)
    }
  }
}
