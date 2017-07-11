#![allow(dead_code)]

extern crate core;
extern crate hyper;
extern crate hyper_tls;
extern crate ws;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate either;
extern crate futures;
//extern crate ctrlc;
extern crate route_recognizer;
//extern crate url;

mod auth;
mod comm;
mod diff;
mod instance;
//mod leaderboard;
//mod mm;
mod rules;

use rules::config::{JsonToGraphConverter, read_json_config};
use instance::access::mem::MemoryAccessor;

use comm::router::*;

fn main() {
  let json_config = read_json_config(std::path::Path::new("./src/config/example.json")).expect("Couldn't read json config!");
  let converter = JsonToGraphConverter::new(json_config);
  let graph = converter.convert().expect("Couldn't convert json config!");

  let accessor = MemoryAccessor::new();

  comm::http::start(8080, accessor).unwrap_or_else(|e| println!("HTTP Error: {:?}", e));
}