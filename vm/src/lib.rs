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

pub mod ast;
pub mod compile;
pub use compile::compile_graph;
