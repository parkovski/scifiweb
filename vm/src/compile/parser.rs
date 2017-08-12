use std::sync::Arc;
use std::path::PathBuf;
use super::lexer::*;
use ast::*;

struct Parser {
  last_token_span: TokenSpan,
}

impl Parser {
  pub fn new(program: &str, filename: PathBuf) -> Self {
    Parser { last_token_span: TokenSpan::new(Arc::new(filename)) }
  }
}
