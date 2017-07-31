use std::{fmt, io};
use std::error::Error;
use serde::de;
use serde_json;

#[derive(Debug)]
pub enum JsonError {
  Serde(serde_json::Error),
  Io(io::Error),
  Deserialize(String),
}

impl fmt::Display for JsonError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    write!(f, "{}", self.description())
  }
}

impl Error for JsonError {
  fn description(&self) -> &str {
    match *self {
      JsonError::Serde(ref e) => e.description(),
      JsonError::Io(ref e) => e.description(),
      JsonError::Deserialize(ref s) => s.as_str(),
    }
  }
}

impl de::Error for JsonError {
  fn custom<T>(msg: T) -> Self
  where
    T: fmt::Display,
  {
    JsonError::Deserialize(format!("{}", msg))
  }
}

impl From<serde_json::Error> for JsonError {
  fn from(error: serde_json::Error) -> Self {
    JsonError::Serde(error)
  }
}

impl From<io::Error> for JsonError {
  fn from(error: io::Error) -> Self {
    JsonError::Io(error)
  }
}

#[derive(Debug)]
pub struct FormatError(String);

impl FormatError {
  pub fn new(expected: &str) -> Self {
    FormatError(format!("Expected {}", expected))
  }
}

impl fmt::Display for FormatError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(&self.0)
  }
}

impl Error for FormatError {
  fn description(&self) -> &str {
    &self.0
  }
}
