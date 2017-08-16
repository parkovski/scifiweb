#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![recursion_limit = "1024"]

#[macro_use]
extern crate nom;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate fxhash;
#[macro_use]
extern crate error_chain;

mod ast;
mod compile;
use compile::compile_graph;

use std::fs::File;
use std::io::Read;

fn main() {
  let mut file = String::new();
  File::open("./test.scifi").unwrap().read_to_string(&mut file);
  compile::compile_graph(&file);
}