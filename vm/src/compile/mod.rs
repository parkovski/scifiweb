mod lexer;
mod parser_rd;
mod token;

pub use self::token::{TokenSpan, TokenValue};

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::path::Path;
use ast::Ast;
use util::graph_cell::{GraphCell, GraphRefMut};

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

  use std::sync::Arc;
  use nom;
  use ast;
  use super::Placeholder;
  use super::token::{TokenValue, TokenSpan};

  error_chain! {
    errors {
      Nom(span: TokenSpan) {
        description("nom/lexer error")
        display("{}: lexer error", &span)
      }

      UnclosedString(span: Placeholder<TokenSpan>) {
        description("unclosed string")
        display("{}: unclosed string", &span)
      }

      Unexpected(token: TokenValue<Arc<str>>) {
        description("unexpected token")
        display("{}: unexpected token '{}'", token.span(), token.value())
      }

      Expected(expected: String, found: TokenValue<Arc<str>>) {
        description("expected token not found")
        display("{}: expected '{}', found '{}'", found.span(), &expected, found.value())
      }

      Syntax(message: String, location: TokenSpan) {
        description("syntax error")
        display("{}: syntax error {}", &location, &message)
      }

      InvalidOperation(operation: &'static str, location: TokenSpan) {
        description("invalid operation")
        display("{}: invalid operation: {}", &location, operation)
      }

      IntegerOutOfRange(integer: TokenValue<i64>, reason: &'static str) {
        description("integer out of range")
        display("{}: integer '{}' out of range: {}", integer.span(), integer.value(), reason)
      }
    }

    foreign_links {
      Io(::std::io::Error);
      Ast(ast::AstError);
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

pub fn compile_file<'a>(filename: &Path) -> ParseResult<Box<GraphCell<Ast<'a>>>> {
  parser_rd::Parser::parse(filename)
}

pub fn compile_string<'a>(
  filename: &Path,
  program: &str,
  ast: GraphRefMut<'a, Ast<'a>>
) -> ParseResult<()>
{
  parser_rd::Parser::parse_str(filename, program, ast)
}
