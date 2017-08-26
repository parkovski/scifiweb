mod lexer;
mod parser_rd;
mod token;

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::Path;

/// In the lexer, tokens are not created until
/// after the errors are.
#[derive(Debug)]
pub struct Placeholder<T: Debug + Display> {
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

impl<T: Debug + Display> Display for Placeholder<T> {
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
  // Why do I need to do this error_chain?
  #![allow(unused_doc_comment)]

  use nom;
  use super::Placeholder;
  use super::token::TokenSpan;

  error_chain! {
    errors {
      UnclosedString(span: Placeholder<TokenSpan>) {
        description("unclosed string")
        display("unclosed string at {}", span)
      }

      UnexpectedToken(token: String) {
        description("unexpected token")
        display("unexpected token {}", token)
      }

      Nom(span: TokenSpan) {
        description("nom")
        display("nom error at {}", span)
      }

      Expected(message: String) {
        description("expected token not found")
        display("{}", message)
      }
    }

    foreign_links {
      Io(::std::io::Error);
    }
  }

  impl ErrorKind {
    pub fn into_nom<I, O>(self) -> nom::IResult<I, O, Error> {
      nom::IResult::Error(nom::ErrorKind::Custom(self.into()))
    }
  }

  impl Error {
    pub fn into_nom<I, O>(self) -> nom::IResult<I, O, Error> {
      nom::IResult::Error(nom::ErrorKind::Custom(self))
    }

    pub fn from_nom(nom_error: nom::ErrorKind<Error>, span: &TokenSpan) -> Self {
      match nom_error {
        nom::ErrorKind::Custom(e) => e,
        // No sense matching anything since
        // they're all turned into Fix.
        _ => ErrorKind::Nom(span.clone()).into(),
      }
    }
  }
}

pub use self::parse_errors::{
  Error as ParseError,
  ErrorKind as ParseErrorKind,
  Result as ParseResult,
  ResultExt as ParseResultExt,
};

pub fn compile_graph(filename: &Path) -> ParseResult<::ast::Ast> {
  parser_rd::Parser::parse(filename)
}
