#![cfg_attr(not(feature = "cargo-clippy"), allow(unknown_lints))]

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
//extern crate ctrlc;
//extern crate docopt;
//#[macro_use]
//extern crate error_chain;
extern crate sf_model;
extern crate sf_model_mem;
extern crate sf_http_server;
extern crate sf_util;

mod config;

use std::path::Path;
use self::config::Config;
use sf_model::rules::config::{read_json_rules, JsonToGraphConverter};
use sf_model_mem::MemoryAccessor;

fn main() {
  sf_model::initialize();
  let _ = sf_util::logger::init();

  let config =
    Config::read(Path::new("./config/example_config.json")).expect("Couldn't read config");

  let json_rules =
    read_json_rules(Path::new("./config/example_rules.json")).expect("Couldn't read json rules");
  let converter = JsonToGraphConverter::new(json_rules);
  let _graph = converter.convert().expect("Couldn't convert json rules");

  let accessor = MemoryAccessor::new();

  sf_http_server::start(config.http_server_addr.as_str(), accessor)
    .unwrap_or_else(|e| error!("HTTP Error: {}", e));
}
