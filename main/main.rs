#![cfg_attr(not(feature = "cargo-clippy"), allow(unknown_lints))]

extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
//extern crate ctrlc;
extern crate docopt;
extern crate scifi_model as model;
extern crate scifi_model_mem as model_mem;
extern crate scifi_http_server as http_server;
extern crate scifi_vm as vm;
extern crate scifi_util as util;

mod config;

use std::path::Path;
use std::default::Default;
use docopt::Docopt;
use log::LogLevelFilter;
use model_mem::MemoryAccessor;
use self::config::{Config, DEFAULT_CONFIG_PATH};

const USAGE: &'static str = "
SciFiWeb Game Management Server

Usage:
  scifiweb [options]
  scifiweb build <input.scifi> [options]
  scifiweb console
  scifiweb --help

Options:
  -c <file> --config=<file>      Specify the server configuration file.
  -o <output>                    Specify the output directory for a build.
  -t <target> --target=<target>  Specify the build target.
                                 Valid targets: csharp, sql, initdb.
  -l <level> --log=<level>       Set the log level.
                                 Valid levels: trace, debug, info, warn, error, off.
";

#[derive(Deserialize, Debug)]
enum Target {
  CSharp,
  Sql,
  InitDb,
}

#[derive(Deserialize, Debug)]
enum LogLevel {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
  Off,
}

impl LogLevel {
  fn into_log_type(self) -> LogLevelFilter {
    use LogLevel::*;
    match self {
      Trace => LogLevelFilter::Trace,
      Debug => LogLevelFilter::Debug,
      Info => LogLevelFilter::Info,
      Warn => LogLevelFilter::Warn,
      Error => LogLevelFilter::Error,
      Off => LogLevelFilter::Off,
    }
  }
}

#[cfg(debug_assertions)]
fn default_log_level() -> LogLevel {
  LogLevel::Trace
}

#[cfg(not(debug_assertions))]
fn default_log_level() -> LogLevel {
  LogLevel::Info
}

#[derive(Deserialize, Debug)]
struct Args {
  cmd_build: bool,
  cmd_console: bool,
  flag_config: Option<String>,
  flag_output: Option<String>,
  flag_target: Option<Target>,
  flag_log: Option<LogLevel>,
  arg_input_scifi: Option<String>,
}

fn main() {
  let args: Args = Docopt::new(USAGE)
    .and_then(|d| d.deserialize())
    .unwrap_or_else(|e| e.exit());

  let mut warn_default_config = false;
  let config_path = args.flag_config.as_ref().map(String::as_str).unwrap_or(DEFAULT_CONFIG_PATH);
  let config = Config::read(Path::new(config_path)).unwrap_or_else(|_| {
    warn_default_config = true;
    Config::default()
  });

  let log_level = args.flag_log.unwrap_or(default_log_level()).into_log_type();
  let _ = util::logger::init(&["scifi"], log_level, &config.time_format);

  if warn_default_config {
    warn!("Couldn't read '{}', using default config.", config_path);
  }

  if args.cmd_build {
    match vm::compile_graph(Path::new("./vm/dot_scifi/simple.scifi")) {
      Ok(_) => info!("Loaded program"),
      Err(e) => error!("{}", e),
    }
  } else {
    model::initialize();
    let accessor = MemoryAccessor::new();
    http_server::start(config.http_server_addr.as_str(), accessor)
      .unwrap_or_else(|e| error!("HTTP Error: {}", e));
  }
}
