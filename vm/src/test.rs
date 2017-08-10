#[macro_use]
extern crate nom;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate fxhash;

mod compile;
use compile::compile_graph;

fn main() {
  compile::compile_graph("test user set of user: `user 'hello '' world'");
}