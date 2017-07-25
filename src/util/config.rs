use std::path::Path;
use std::fs::File;

use serde_json;

use util::error::JsonError;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
  pub http_server_addr: String,
  pub ws_server_addr: String,
}

impl Config {
  pub fn read(filename: &Path) -> Result<Self, JsonError> {
    Ok(serde_json::from_reader(File::open(filename)?)?)
  }
}