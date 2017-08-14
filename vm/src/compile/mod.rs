mod lexer;
mod parser;
mod token;

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use self::lexer::next_token;
use self::token::*;

/// In the lexer, tokens are not created until
/// after the errors are.
#[derive(Debug)]
struct Placeholder<T: Debug + Display> {
  value: Option<T>,
}

impl<T: Debug + Display> Placeholder<T> {
  pub fn new() -> Self {
    Placeholder { value: None }
  }

  pub fn fill(&mut self, value: T) {
    self.value = Some(value)
  }

  pub fn as_ref(&self) -> Option<&T> {
    self.value.as_ref()
  }

  pub fn as_mut(&mut self) -> Option<&mut T> {
    self.value.as_mut()
  }

  pub fn into_inner(self) -> Option<T> {
    self.value
  }
}

impl<T: Display> Display for Placeholder {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    match self.value {
      Some(ref value) => write!(f, "{}", value),
      None => {
        debug!("Displaying empty placeholder");
        write!(f, "<Empty>")
      }
    }
  }
}

mod parse_errors {
  error_chain! {
    errors {
      UnclosedString(span: Placeholder<TokenSpan>) {
        description("unclosed string"),
        display("unclosed string at {}", &span),
      }

      UnexpectedToken(token: Token) {
        description("unexpected token"),
        display("unexpected token {}", &token),
      }
    }
  }

  use nom;
  impl ErrorKind {
    pub fn into_nom<I, O>(self) -> nom::IResult<I, O, Error> {
      nom::IResult::Error(nom::ErrorKind::Custom(self.into()))
    }
  }
}

pub use self::parse_errors::{
  Error as ParseError,
  ErrorKind as ParseErrorKind,
  Result as ParseResult,
  ResultExt as ParseResultExt,
};

pub fn compile_graph(program: &str) {
  let mut span = TokenSpan::new(::std::sync::Arc::new(::std::path::PathBuf::new()));
  let mut input = program.as_bytes();
  loop {
    let r = lexer::next_token(input, &span).unwrap();
    let token = r.1;
    input = r.0;
    println!("{}", &token);
    if token.kind == TokenKind::Eof {
      break;
    }
    span = token.span;
  }
}
