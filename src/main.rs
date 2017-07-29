#![allow(dead_code)]

#![feature(try_trait)]
#![feature(conservative_impl_trait)]

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
extern crate ctrlc;
extern crate route_recognizer;
extern crate url;
#[macro_use]
extern crate log;
extern crate termcolor;
extern crate atomic;
extern crate crossbeam;

mod auth;
mod comm;
//mod diff;
mod instance;
//mod leaderboard;
//mod mm;
mod rules;
#[macro_use]
mod util;

use std::path::Path;

use util::config::Config;
use rules::config::{JsonToGraphConverter, read_json_rules};
use instance::access::mem::MemoryAccessor;

fn main() {
  let _ = util::log::init();
 
  let config = Config::read(Path::new("./config/example_config.json")).expect("Couldn't read config");

  let json_rules = read_json_rules(Path::new("./config/example_rules.json")).expect("Couldn't read json rules");
  let converter = JsonToGraphConverter::new(json_rules);
  let graph = converter.convert().expect("Couldn't convert json rules");

  let accessor = MemoryAccessor::new();

  comm::http::start(config.http_server_addr.as_str(), accessor).unwrap_or_else(|e| error!("HTTP Error: {}", e));
}