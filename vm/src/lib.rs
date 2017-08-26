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
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate scifi_util as util;

pub mod ast;
pub mod compile;
pub use compile::compile_graph;
