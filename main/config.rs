use std::path::Path;
use std::fs::File;
use std::default::Default;
use serde_json;
use model::rules::error::JsonError;

pub const DEFAULT_CONFIG_PATH: &'static str = "./config/config.json";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Config {
  pub http_server_addr: String,
  pub ws_server_addr: String,
  pub time_format: String,
}

impl Config {
  pub fn read(filename: &Path) -> Result<Self, JsonError> {
    Ok(serde_json::from_reader(File::open(filename)?)?)
  }
}

impl Default for Config {
  fn default() -> Self {
    Config {
      http_server_addr: "127.0.0.1:8085".into(),
      ws_server_addr: "127.0.0.1:8086".into(),
      time_format: "%F %T%z".into(),
    }
  }
}
