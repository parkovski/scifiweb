mod lexer;
mod parser;

use self::lexer::*;

pub fn compile_graph(program: &str) {
  let mut span = TokenSpan::new(::std::sync::Arc::new(::std::path::PathBuf::new()));
  let mut input = program.as_bytes();
  loop {
    let r = lexer::next_token(input, &span).unwrap();
    let token = r.1;
    input = r.0;
    println!("{:?}", token.kind);
    if token.kind == TokenKind::Eof {
      break;
    }
    span = token.span;
  }
}
