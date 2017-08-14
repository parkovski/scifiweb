use std::sync::Arc;
use std::path::PathBuf;
use std::cmp::Ordering;
use nom::IResult;
use fxhash::FxHashMap;
use super::token::*;
use super::lexer::next_token;
use super::parse_errors::*;
use ast::*;

/// $kind: TokenKind with a value (ex. Identifier(String)),
/// returns a TokenValue with the inner value.
macro_rules! tval {
  ($i:expr, $kind:ident) => (
    match self.next_token($i) {
      IResult::Done(inp, tok) => match tok.kind {
        TokenKind::$kind(value) => {
          IResult::Done(
            inp,
            TokenValue::new(value, stringify!($kind), tok.span)
          )
        }
        t @ _ => ErrorKind::UnexpectedToken(t).into_nom(),
      }
      IResult::Incomplete(i) => IResult::Incomplete(i),
      IResult::Error(e) => IResult::Error(e),
    }
  )
}

/// Filters a token result to only match the given token.
/// If the token has an inner value it can be ignored with an _.
macro_rules! tok {
  ($i:expr, $kind:ident) => (
    match self.next_token($i) {
      IResult::Done(inp, t @ Token { kind: TokenKind::$kind, .. })
        => IResult::Done(inp, t),
      other @ _ => other,
    }
  )
}

macro_rules! parser_m {
  ($name:ident($self_:ident) -> $o:ty, $submac:ident!( $($args:tt)* )) => (
    method!($name<Parser, &[u8], $o, Error>, $self_, $submac)
  );
  ($name:ident(mut $self_:ident) -> $o:ty, $submac:ident!( $($args:tt)* )) => (
    method!($name<Parser, &[u8], $o, Error>, mut $self_, $submac)
  );
  ($name:ident($self_:ident, $i:ty) -> $o:ty, $submac:ident!( $($args:tt)* )) => (
    method!($name<Parser, $i, $o, Error>, $self_, $submac)
  );
  ($name:ident(mut $self_:ident, $i:ty) -> $o:ty, $submac:ident!( $($args:tt)* )) => (
    method!($name<Parser, $i, $o, Error>, mut $self_, $submac)
  )
}

type PResult<I, O> = IResult<I, O, Error>;

struct OrderedTokenSpan {
  inp_pointer: *const [u8],
  span: TokenSpan,
}

impl OrderedTokenSpan {
  pub fn new(inp: &[u8], span: TokenSpan) -> Self {
    OrderedTokenSpan { inp_pointer: inp as *const _, span }
  }
}

impl PartialEq for OrderedTokenSpan {
  fn eq(&self, other: &Self) -> bool {
    self.inp_pointer == other.inp_pointer
  }
}

impl Eq for OrderedTokenSpan {}

impl Ord for OrderedTokenSpan {
  fn cmp(&self, other: &Self) -> Ordering {
    self.inp_pointer.cmp(other.inp_pointer)
  }
}

impl PartialOrd for OrderedTokenSpan {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

struct Parser {
  file: Arc<PathBuf>,
  token_spans: Vec<OrderedTokenSpan>,
}

impl Parser {
  pub fn new(filename: PathBuf) -> Self {
    Parser {
      file: Arc::new(filename),
      token_spans: Vec::new(),
    }
  }

  pub fn parse(mut self, program: &str) -> Result<Ast, Err> {
    let inp = program.as_bytes();
    self.token_spans.push(OrderedTokenSpan::new(inp, TokenSpan::new(self.file.clone())));
    self.parse_program(inp).1.to_result()
  }

  fn next_token<'a>(&mut self, inp: &'a [u8]) -> PResult<&'a [u8], Token> {
    let (prev_span, should_insert) = self.find_previous_token_span(inp);
    let result = next_token(inp, &prev_span);
    if should_insert {
      if let IResult::Done(_, Token { ref span, .. }) = result {
        self.token_spans.push(OrderedTokenSpan::new(inp, span.clone()));
      }
    }
    result
  }

  /// Returns the previous span and a flag indicating whether
  /// the new span should be inserted.
  fn find_previous_token_span<'a, 'b: 'a>(&'a self, inp: &'b [u8])
    -> (&'a TokenSpan, bool)
  {
    let span = TokenSpan::new(self.file.clone());
    let ord_span = OrderedTokenSpan::new(inp, span);
    // Most likely it will be at the end.
    let last_span = &self.token_spans[self.token_spans.len()].span;
    if ord_span > last_span {
      return (last_span, true);
    }
    match self.token_spans.binary_search(&span) {
      Ok(index) => {
        let index = if index == 0 { 0 } else { index - 1 };
        (&self.token_spans[index].span, false)
      }
      Err(_) => unreachable!("Span should be the last, which is handled earlier"),
    }
  }

  /// When we're sure there will be no more backtracking
  /// (after a top level item is parsed), clear out the
  /// old spans for faster searches.
  fn clear_token_spans(&mut self) {
    debug_assert!(
      self.token_spans.len() > 1,
      "There should always be at least 1 stored TokenSpan"
    );
    let keep_last = self.token_spans.pop().unwrap();
    self.token_spans.clear();
    self.token_spans.push(keep_last);
  }

  parser_m!(parse_program(self) -> Ast,
    map!(
      fold_many0!(
        alt_complete!(
          call_m!(parse_include)
          | call_m!(parse_item_definition)
        ),
        Vec::new(),
        |mut acc: Vec<_>, item| {
          acc.push(item);
          acc
        }
      ),
      Ast::new
    )
  );

  parser_m!(parse_include(mut self) -> Box<TopLevelItem>,
    do_parse!(
      tok!(Include) >>
      file: tval!(String) >>
      tok!(Semicolon) >>
      Include::new(file)
    )
  );

  parser_m!(parse_item_definition(mut self) -> Box<TopLevelItem>,
    do_parse!(
      label: tval!(Label) >>
      switch!()
    )
  );
}
