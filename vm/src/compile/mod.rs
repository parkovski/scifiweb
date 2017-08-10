mod lexer;
mod parser;

pub fn compile_graph(program: &str) {
  println!("tokens:\n{:?}", lexer::lex(program));
}
