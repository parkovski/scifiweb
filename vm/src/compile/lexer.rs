use std::str;
use nom::{self, IResult};
use super::Placeholder;
use super::token::*;
use super::parse_errors::*;

macro_rules! lexfn {
  ($name:ident -> $ty:ty, $submac:ident!( $($args:tt)* )) => (
    #[allow(unused_variables)]
    fn $name<'a>(i: &'a [u8]) -> IResult<&'a [u8], $ty, Error> {
      match $submac!(i, $($args)*) {
        IResult::Done(ii, o) => IResult::Done(ii, o),
        IResult::Incomplete(n) => IResult::Incomplete(n),
        IResult::Error(_) => IResult::Error(nom::ErrorKind::Fix),
      }
    }
  );
  (nofixerr: $name:ident -> $ty:ty, $submac:ident!( $($args:tt)* )) => (
    #[allow(unused_variables)]
    fn $name<'a>(i: &'a [u8]) -> IResult<&'a [u8], $ty, Error> {
      $submac!(i, $($args)*)
    }
  );
  (nofixerr($e:ty): $name:ident -> $ty:ty, $submac:ident!( $($args:tt)* )) => (
    #[allow(unused_variables)]
    fn $name<'a>(i: &'a [u8]) -> IResult<&'a [u8], $ty, $e> {
      $submac!(i, $($args)*)
    }
  );
}

lexfn!(op_semicolon -> TokenKind<'a>,
  do_parse!(tag!(";") >> (TokenKind::Semicolon))
);
lexfn!(op_dot -> TokenKind<'a>,
  do_parse!(tag!(".") >> (TokenKind::Dot))
);
lexfn!(op_comma -> TokenKind<'a>,
  do_parse!(tag!(",") >> (TokenKind::Comma))
);
lexfn!(op_lparen -> TokenKind<'a>,
  do_parse!(tag!("(") >> (TokenKind::LParen))
);
lexfn!(op_rparen -> TokenKind<'a>,
  do_parse!(tag!(")") >> (TokenKind::RParen))
);
lexfn!(op_lsquarebracket -> TokenKind<'a>,
  do_parse!(tag!("[") >> (TokenKind::LSquareBracket))
);
lexfn!(op_rsquarebracket -> TokenKind<'a>,
  do_parse!(tag!("]") >> (TokenKind::RSquareBracket))
);
lexfn!(op_minus -> TokenKind<'a>,
  do_parse!(tag!("-") >> (TokenKind::Minus))
);
lexfn!(op_plus -> TokenKind<'a>,
  do_parse!(tag!("+") >> (TokenKind::Plus))
);
lexfn!(op_multiply -> TokenKind<'a>,
  do_parse!(tag!("*") >> (TokenKind::Multiply))
);
lexfn!(op_divide -> TokenKind<'a>,
  do_parse!(tag!("/") >> (TokenKind::Divide))
);
lexfn!(op_caret -> TokenKind<'a>,
  do_parse!(tag!("^") >> (TokenKind::Caret))
);
lexfn!(op_equal -> TokenKind<'a>,
  do_parse!(tag!("=") >> (TokenKind::Equal))
);
lexfn!(op_notequal -> TokenKind<'a>,
  do_parse!(tag!("!=") >> (TokenKind::NotEqual))
);
lexfn!(op_less -> TokenKind<'a>,
  do_parse!(tag!("<") >> (TokenKind::Less))
);
lexfn!(op_greater -> TokenKind<'a>,
  do_parse!(tag!(">") >> (TokenKind::Greater))
);
lexfn!(op_lessequal -> TokenKind<'a>,
  do_parse!(tag!("<=") >> (TokenKind::LessEqual))
);
lexfn!(op_greaterequal -> TokenKind<'a>,
  do_parse!(tag!(">=") >> (TokenKind::GreaterEqual))
);
lexfn!(op_percentsign -> TokenKind<'a>,
  do_parse!(tag!("%") >> (TokenKind::PercentSign))
);
lexfn!(op_exclamation -> TokenKind<'a>,
  do_parse!(tag!("!") >> (TokenKind::Exclamation))
);

lexfn!(operator -> TokenKind<'a>,
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

lexfn!(identifier -> TokenKind<'a>,
  alt!(regular_identifier | escaped_identifier)
);

lexfn!(regular_identifier -> TokenKind<'a>,
  do_parse!(
    chars: recognize!(
      do_parse!(
        verify!(take!(1), |c: &[u8]| is_identifier_begin(c[0])) >>
        take_while!(is_identifier_char) >>
        (())
      )
    ) >>
    colon: opt!(complete!(tag!(":"))) >>
    (get_id_or_keyword(chars, colon.is_some()))
  )
);

lexfn!(escaped_identifier -> TokenKind<'a>,
  do_parse!(
    s: preceded!(tag!("`"), take_while1!(is_identifier_char)) >>
    (TokenKind::Identifier(str::from_utf8(s).unwrap()))
  )
);

fn get_number<'a>(mut chars: &'a [u8]) -> TokenKind<'a> {
  let is_pct = if chars[chars.len() - 1] == b'%' {
    chars = &chars[0..chars.len() - 1];
    true
  } else {
    false
  };
  let s = str::from_utf8(chars).unwrap();
  let is_dec = s.find('.').is_some();

  if is_pct {
    TokenKind::Percentage(s.parse().unwrap())
  } else if is_dec {
    TokenKind::Decimal(s.parse().unwrap())
  } else {
    TokenKind::Integer(s.parse().unwrap())
  }
}

lexfn!(number -> TokenKind<'a>,
  do_parse!(
    chars: recognize!(
      do_parse!(
        take_while1!(is_integer) >>
        opt!(recognize!(preceded!(tag!("."), take_while1!(is_integer)))) >>
        opt!(complete!(tag!("%"))) >>
        (())
      )
    ) >>
    (get_number(chars))
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
      ErrorKind::UnclosedString(Placeholder::new()).into_nom()
    }
  } else if inp[0] == b'\n' {
    ErrorKind::UnclosedString(Placeholder::new()).into_nom()
  } else if len > 0 {
    IResult::Done(&inp[1..], &inp[0..1])
  } else {
    ErrorKind::UnclosedString(Placeholder::new()).into_nom()
  }
}

lexfn!(nofixerr(Error): string -> TokenKind<'a>,
  do_parse!(
    s: delimited!(
      fix_error!(Error, tag!("'")),
      recognize!(fold_many0!(
        call!(string_mid),
        (),
        |_: (), _| {}
      )),
      fix_error!(Error, tag!("'"))
    ) >>
    (TokenKind::String(str::from_utf8(s).unwrap()))
  )
);

lexfn!(line_comment -> &'a [u8],
  recognize!(do_parse!(
    char!('#') >> is_not!("\n") >> ()
  ))
);

lexfn!(nofixerr: whitespace -> (usize, usize),
  fold_many0!(
    fix_error!(Error,
      alt_complete!(
        fix_error!(Error, recognize!(one_of!(" \t\r\n")))
        | line_comment
      )
    ),
    (0, 0),
    |(lines, columns): (usize, usize), item: &[u8]| -> (usize, usize) {
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

lexfn!(invalid -> TokenKind<'a>,
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
lexfn!(lex_one_token -> (usize, usize, usize, TokenKind<'a>),
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
  -> IResult<&'a [u8], Token<'a>, Error>
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
    IResult::Error(mut e) => {
      if let nom::ErrorKind::Custom(Error(ErrorKind::UnclosedString(ref mut placeholder), _)) = e {
        // TODO: How to get the actual span for this token?
        placeholder.fill(TokenSpan::with_position(
          last_token_span.filename.clone(),
          last_token_span.line,
          last_token_span.start,
          last_token_span.end,
        ));
      }
      IResult::Error(e)
    }
  }
}
