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
mod options;

use std::path::Path;
use std::fs::File;
use docopt::Docopt;
use model_mem::MemoryAccessor;
use vm::ast::Ast;
use self::config::{Config, DEFAULT_CONFIG_PATH};
use self::options::DebugOptions;

/*extern "C" {
  static __BaseImage: i8;
}*/

const USAGE: &'static str = "
SciFiWeb Game Management Server

Usage:
  scifiweb [options]
  scifiweb init <dir>
  scifiweb build [-t <target>] [options]
  scifiweb run <file> [options]
  scifiweb console [-u <user> (-k <key-file> | -p [<password>])]
  scifiweb --help

Options:
  -C <file> --config=<file>       Specify the server configuration file.
                                  The default is './scifiweb.json'.
  -c <key=value> ...              Override a configuration option.
  -t <target> --target=<target>   Specify the build target.
                                  Valid targets: all, csharp, sql.
  -z <debug-options> ...          Set a debug option.

Command overview:
  (none)      Start a server for the program listed in the configuration file.
  init        Create an initial configuration and source file in <dir>.
  build       Build the specified target.
  run         Run a self-contained program.
  console     Start the interactive console.
";

#[derive(Deserialize, Debug, Copy, Clone, PartialEq, Eq)]
enum Target {
  All,
  CSharp,
  Sql,
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
  cmd_run: bool,
  cmd_console: bool,
  arg_dir: String,
  arg_file: String,
  flag_config: Option<String>,
  flag_c: Vec<String>,
  flag_target: Option<Target>,
  flag_z: DebugOptions,
}

fn init_logger(config: &Config) {
  let _ = util::logger::init(&["scifi"], config.log.level, &config.log.time_format);
}

fn init(dir: &str) {
  let c = Config::default();
  init_logger(&c);
  let path = Path::new(dir).join("scifiweb.json");
  debug!("writing default config to {:?}", path);
  Config::write(&path, &c).unwrap_or_else(|e| {
    error!("{}", e);
  });
}

fn write_ast<'a>(ast: &Ast<'a>) {
  let file = match File::create(Path::new("./ast.json")) {
    Ok(f) => f,
    Err(e) => {
      error!("{}", e);
      return;
    }
  };
  match serde_json::to_writer_pretty(file, &ast) {
    Ok(_) => info!("Wrote ast to ./ast.json"),
    Err(e) => error!("{}", e),
  }
}

fn main() {
  #[cfg(windows)]
  util::win_bt::set_panic_hook();

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
    build(&config.program, args.flag_z.save_ast);
  } else if args.cmd_run {
    trace!("Running {}", args.arg_file);
    build(&args.arg_file, args.flag_z.save_ast);
  } else {
    model::initialize();
    let accessor = MemoryAccessor::new();
    http_server::start(config.server.http_addr.as_str(), accessor)
      .unwrap_or_else(|e| error!("HTTP Error: {}", e));
  }
}

fn build(filename: &str, save_ast: bool) {
  match vm::compile_file(Path::new(filename)) {
    Ok(ast) => {
      info!("Loaded program.");
      if save_ast {
        write_ast(&ast.awake());
      }
    }
    Err(e) => error!("{}", e),
  }
}
