#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![recursion_limit = "1024"]
#![feature(conservative_impl_trait)]

#[macro_use]
extern crate nom;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate fxhash;
#[macro_use]
extern crate error_chain;
extern crate scifi_util as util;

mod ast;
mod compile;
use compile::compile_graph;

use std::path::Path;

fn main() {
  let _ = util::logger::init(&["vmtest", "scifi"]);
  compile::compile_graph(Path::new("./dot_scifi/simple.scifi"));
}