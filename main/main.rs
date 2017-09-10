#![cfg_attr(not(feature = "cargo-clippy"), allow(unknown_lints))]

extern crate serde;
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
use docopt::Docopt;
use model_mem::MemoryAccessor;
use self::config::{Config, DEFAULT_CONFIG_PATH};

const USAGE: &'static str = "
SciFiWeb Game Management Server

Usage:
  scifiweb [options]
  scifiweb init <dir>
  scifiweb build [-t <target>] [options]
  scifiweb console [-u <user> (-k <key-file> | -p [<password>])]
  scifiweb --help

Options:
  -C <file> --config=<file>       Specify the server configuration file.
                                  The default is './scifiweb.json'.
  -c <key=value> ...              Override a configuration option.
  -t <target> --target=<target>   Specify the build target.
                                  Valid targets: all, csharp, sql, initdb.

Command overview:
  (none)      Start a server for the program listed in the configuration file.
  init        Create an initial configuration and source file in <dir>.
  build       Build the specified target.
  console     Start the interactive console.
";

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
enum Target {
  All,
  CSharp,
  Sql,
  InitDb,
}

impl Default for Target {
  fn default() -> Self {
    Target::All
  }
}

#[derive(Deserialize, Debug)]
struct Args {
  cmd_init: bool,
  cmd_build: bool,
  cmd_console: bool,
  arg_dir: String,
  flag_config: Option<String>,
  flag_c: Vec<String>,
  flag_target: Option<Target>,
}

fn init_logger(config: &Config) {
  let _ = util::logger::init(&["scifi"], config.log.level, &config.log.time_format);
}

fn init(dir: &str) {
  let c = Config::default();
  init_logger(&c);
  let path = Path::new(dir).join("scifiweb.json");
  debug!("{:?}", path);
  Config::write(&path, &c).unwrap_or_else(|e| {
    error!("{}", e);
  });
}

fn main() {
  let args: Args = Docopt::new(USAGE)
    .and_then(|d| d.deserialize())
    .unwrap_or_else(|e| e.exit());

  if args.cmd_init {
    init(&args.arg_dir);
    return;
  }

  let mut warn_default_config = false;
  let config_path = args.flag_config.as_ref().map(String::as_str).unwrap_or(DEFAULT_CONFIG_PATH);
  let config = Config::read(Path::new(config_path)).unwrap_or_else(|_| {
    warn_default_config = true;
    Config::default()
  });

  init_logger(&config);

  debug!("{:?}", args);

  if warn_default_config {
    warn!("Couldn't read '{}', using default config.", config_path);
  }

  if args.cmd_build {
    trace!("Starting build for {}, target {:?}", &config.program, args.flag_target);
    match vm::compile_graph(Path::new(&config.program)) {
      Ok(_) => info!("Loaded program."),
      Err(e) => error!("{}", e),
    }
  } else {
    model::initialize();
    let accessor = MemoryAccessor::new();
    http_server::start(config.server.http_addr.as_str(), accessor)
      .unwrap_or_else(|e| error!("HTTP Error: {}", e));
  }
}
