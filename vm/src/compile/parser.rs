use std::sync::Arc;
use std::path::PathBuf;
use std::cmp::Ordering;
use std::cell::Cell;
use nom::IResult;
use super::token::*;
use super::lexer::next_token;
use super::parse_errors::*;
use ast::*;

macro_rules! e {
  ($submac:ident!( $($args:tt)* )) => (
    fix_error!(Error, $submac!($($args)*))
  )
}

/// $kind: TokenKind with a value (ex. Identifier(String)),
/// returns a TokenValue with the inner value.
macro_rules! tval {
  ($i:expr, $self_:ident <- $kind:ident) => (
    match $self_.next_token($i) {
      IResult::Done(inp, tok) => {
        if let TokenKind::$kind(value) = tok.kind {
          IResult::Done(
            inp,
            TokenValue::new(value, stringify!($kind), tok.span)
          )
        } else {
          ErrorKind::UnexpectedToken(tok).into_nom()
        }
      }
      IResult::Incomplete(i) => IResult::Incomplete(i),
      IResult::Error(e) => IResult::Error(e),
    }
  )
}

/// Filters a token result to only match the given token.
macro_rules! tok {
  ($i:expr, $self_:ident <- $kind:ident) => (
    match $self_.next_token($i) {
      IResult::Done(inp, t @ Token { kind: TokenKind::$kind, .. })
        => IResult::Done(inp, t),
      other @ _ => other,
    }
  );
  ($i:expr, $self_:ident <- $kind:ident($inner:pat)) => (
    match $self_.next_token($i) {
      IResult::Done(inp, t @ Token { kind: TokenKind::$kind($inner), .. })
        => IResult::Done(inp, t),
      other @ _ => other,
    }
  );
}

/// Convenience.
macro_rules! kwd {
  ($i:expr, $self_:ident <- $kwd:ident) => (
    tok!($i, $self_ <- Keyword(Keyword::$kwd))
  )
}

macro_rules! parser_m {
  /*($name:ident($self_:ident) -> $o:ty, $submac:ident!( $($args:tt)* )) => (
    method!($name<Parser, &[u8], $o, Error>, $self_, $submac!($($args)*));
  );*/
  ($name:ident($self_:ident $(, $arg:ident: $ty:ty)*) -> $o:ty, $submac:ident!( $($args:tt)* )) => (
    #[allow(unused_variables)]
    fn $name<'a>($self_, i: &'a [u8] $(, $arg: $ty)*) -> (Parser, PResult<'a, $o>) {
      let result = $submac!(i, $($args)*);
      ($self_, result)
    }
  );
  ($name:ident(mut $self_:ident $(, $arg:ident: $ty:ty)*) -> $o:ty, $submac:ident!( $($args:tt)* )) => (
    #[allow(unused_variables)]
    fn $name<'a>(mut $self_, i: &'a [u8] $(, $arg: $ty)*) -> (Parser, PResult<'a, $o>) {
      let result = $submac!(i, $($args)*);
      ($self_, result)
    }
  );
}

macro_rules! box_trait {
  ($struc:ty, $trait_:ty, $ex:expr) => ((|s:$struc| -> Box<$trait_> { Box::new(s) })($ex))
}

pub type PResult<'a, O> = IResult<&'a [u8], O, Error>;

#[derive(Debug)]
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
    self.inp_pointer.cmp(&other.inp_pointer)
  }
}

impl PartialOrd for OrderedTokenSpan {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

pub struct Parser {
  file: Arc<PathBuf>,
  token_spans: Vec<OrderedTokenSpan>,
  last_span_index: Cell<usize>,
}

impl Parser {
  pub fn new(filename: PathBuf) -> Self {
    Parser {
      file: Arc::new(filename),
      token_spans: Vec::new(),
      last_span_index: Cell::new(0),
    }
  }

  pub fn parse<'a>(mut self, program: &'a str) -> PResult<'a, Ast> {
    let inp = program.as_bytes();
    self.token_spans.push(OrderedTokenSpan::new(inp, TokenSpan::new(self.file.clone())));
    self.parse_program(inp).1
  }

  fn next_token<'a>(&mut self, inp: &'a [u8]) -> PResult<'a, Token> {
    let (result, should_insert) = {
      let (prev_span, should_insert) = self.find_previous_token_span(inp);
      (next_token(inp, &prev_span), should_insert)
    };
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
    // Most likely it will be the last token parsed.
    let last_span_index = self.last_span_index.get();
    let last_span = &self.token_spans[last_span_index];
    if &ord_span > last_span {
      if last_span_index == self.token_spans.len() - 1 {
        self.last_span_index.set(last_span_index + 1);
        return (&last_span.span, true);
      } else if &ord_span == &self.token_spans[last_span_index + 1] {
        self.last_span_index.set(last_span_index + 1);
        return (&last_span.span, false);
      }
    }
    match self.token_spans.binary_search(&ord_span) {
      Ok(index) => {
        self.last_span_index.set(index);
        let index = if index == 0 { 0 } else { index - 1 };
        (&self.token_spans[index].span, false)
      }
      // Impossible since tokens should move forward one
      // at a time and are always stored in token_spans.
      Err(_) => unreachable!("Span should be the last, which is handled earlier"),
    }
  }

  /// When we're sure there will be no more backtracking
  /// (after a top level item is parsed), clear out the
  /// old spans for faster searches & lower memory usage.
  /// Unsafe because lots of assumptions are made that will
  /// break if a span isn't available.
  unsafe fn clear_token_spans(&mut self) {
    debug_assert!(
      self.token_spans.len() > 1,
      "There should always be at least 1 stored TokenSpan"
    );
    let keep_last = self.token_spans.pop().unwrap();
    self.token_spans.clear();
    self.token_spans.push(keep_last);
    self.last_span_index.set(0);
  }

  parser_m!(parse_program(mut self) -> Ast,
    map!(
      fold_many0!(
        alt_complete!(
          call_m!(self.parse_include)
          | call_m!(self.parse_item_definition)
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
      kwd!(self <- Include) >>
      file: tval!(self <- String) >>
      tok!(self <- Semicolon) >>
      (box_trait!(Include, TopLevelItem, Include::new(file.into_inner())))
    )
  );

  parser_m!(parse_item_definition(mut self) -> Box<TopLevelItem>,
    do_parse!(
      label: tval!(self <- Label) >>
      /*alt_complete!(
        call_m!(parse_user, label)
        | call_m!(parse_collectable, label)
        | call_m!(parse_map, label)
        | call_m!(parse_event, label)
        | call_m!(parse_random, label)
      )*/
      foo: apply_m!(self.parse_user, label) >>
      (foo)
    )
  );

  parser_m!(parse_user(mut self, label: TokenValue<String>) -> Box<TopLevelItem>,
    do_parse!(
      kwd!(self <- User) >>
      tok!(self <- Semicolon) >>
      (box_trait!(User, TopLevelItem, User::new(label.into_inner())))
    )
  );
}
