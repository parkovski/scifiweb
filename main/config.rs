use std::path::Path;
use std::fs::File;
use std::fmt;
use serde::ser::Serializer;
use serde::de::{self, Deserializer, Visitor, Unexpected};
use serde_json;
use log::LogLevelFilter;

pub const DEFAULT_CONFIG_PATH: &'static str = "./scifiweb.json";

fn serialize_log_level<S: Serializer>(level: &LogLevelFilter, serializer: S)
  -> Result<S::Ok, S::Error>
{
  let s = match *level {
    LogLevelFilter::Trace => "trace",
    LogLevelFilter::Debug => "debug",
    LogLevelFilter::Info => "info",
    LogLevelFilter::Warn => "warn",
    LogLevelFilter::Error => "error",
    LogLevelFilter::Off => "off",
  };
  serializer.serialize_str(s)
}

fn deserialize_log_level<'de, D: Deserializer<'de>>(deserializer: D)
  -> Result<LogLevelFilter, D::Error>
{
  struct LogLevelFilterVisitor;
  impl<'de> Visitor<'de> for LogLevelFilterVisitor {
    type Value = LogLevelFilter;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("trace, debug, info, warn, error, or off")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<LogLevelFilter, E> {
      match value {
        "trace" => Ok(LogLevelFilter::Trace),
        "debug" => Ok(LogLevelFilter::Debug),
        "info" => Ok(LogLevelFilter::Info),
        "warn" => Ok(LogLevelFilter::Warn),
        "error" => Ok(LogLevelFilter::Error),
        "off" => Ok(LogLevelFilter::Off),
        _ => Err(E::invalid_value(Unexpected::Str(value), &self)),
      }
    }
  }

  deserializer.deserialize_str(LogLevelFilterVisitor)
}

#[cfg(debug_assertions)]
fn default_log_level() -> LogLevelFilter {
  LogLevelFilter::Trace
}

#[cfg(not(debug_assertions))]
fn default_log_level() -> LogLevelFilter {
  LogLevelFilter::Info
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct LogOpts {
  pub file: Option<String>,
  #[serde(serialize_with = "serialize_log_level")]
  #[serde(deserialize_with = "deserialize_log_level")]
  pub level: LogLevelFilter,
  pub time_format: String,
  pub show_module: bool,
}

impl Default for LogOpts {
  fn default() -> Self {
    LogOpts {
      file: None,
      level: default_log_level(),
      time_format: "%F %T%z".into(),
      show_module: true,
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OutDirs {
  pub cs: String,
  pub sql: String,
}

impl Default for OutDirs {
  fn default() -> Self {
    OutDirs {
      cs: "./out/csharp".into(),
      sql: "./out/sql".into(),
    }
  }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct ServerConfig {
  pub http_addr: String,
  pub https_addr: String,
  pub ws_addr: String,
  pub wss_addr: String,
}

impl Default for ServerConfig {
  fn default() -> Self {
    ServerConfig {
      http_addr: "127.0.0.1:43080".into(),
      https_addr: "127.0.0.1:43081".into(),
      ws_addr: "127.0.0.1:43082".into(),
      wss_addr: "127.0.0.1:43083".into(),
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum DefaultTimeZone {
  UTC,
  Local,
  Offset(i8),
}

// Note: Local is never equal to anything because
// it depends on where the program is run.
impl PartialEq for DefaultTimeZone {
  fn eq(&self, other: &Self) -> bool {
    use self::DefaultTimeZone::*;
    match (*self, *other) {
      (UTC, UTC) => true,
      (UTC, Offset(0)) => true,
      (Offset(0), UTC) => true,
      (Offset(a), Offset(b)) if a == b => true,
      _ => false,
    }
  }
}

impl Default for DefaultTimeZone {
  fn default() -> Self {
    DefaultTimeZone::UTC
  }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default, rename_all="camelCase")]
pub struct Config {
  pub program: String,
  pub server: ServerConfig,
  pub log: LogOpts,
  pub out: OutDirs,
  pub default_time_zone: DefaultTimeZone,
}

impl Config {
  pub fn read(filename: &Path) -> serde_json::Result<Self> {
    use serde::de::Error;
    let file = File::open(filename).map_err(
      |e| serde_json::Error::custom(e)
    )?;
    serde_json::from_reader(file)
  }

  pub fn write(filename: &Path, config: &Config) -> serde_json::Result<()> {
    use serde::ser::Error;
    let file = File::create(filename).map_err(
      |e| serde_json::Error::custom(e)
    )?;
    serde_json::to_writer_pretty(file, config)
  }
}

impl Default for Config {
  fn default() -> Self {
    Config {
      program: "-".into(),
      server: Default::default(),
      log: Default::default(),
      out: Default::default(),
      default_time_zone: Default::default(),
    }
  }
}
