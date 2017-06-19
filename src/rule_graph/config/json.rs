use std::collections::HashMap;
use std::fmt;
use std::io;
use std::path::Path;
use std::fs::File;
use std::marker::PhantomData;
use std::error::Error;
use serde::de::{self, Deserializer, Visitor, MapAccess };
use serde_json;

#[derive(Deserialize, Debug)]
#[serde(untagged)]
pub enum Redemption {
  Event { amount: i32, #[serde(rename = "costEvent")] cost_event: String },
  Collectable {
    amount: i32,
    #[serde(rename = "costCollectable")] cost_collectable: String,
    #[serde(rename = "costAmount")] cost_amount: i32
  },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Upgrade {
  pub level: i32,
  pub cost_collectable: String,
  pub cost_amount: i32,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
pub struct Collectable {
  #[serde(default)]
  pub redemptions: Vec<Redemption>,
  #[serde(default)]
  pub upgrades: Vec<Upgrade>,
}

impl Default for Collectable {
  fn default() -> Self {
    Collectable {
      redemptions: Vec::new(),
      upgrades: Vec::new(),
    }
  }
}

#[derive(Debug)]
pub enum EventTarget {
  Global,
  Profile,
  Group(Option<String>),
}

#[derive(Deserialize, Debug)]
pub struct Event {
  #[serde(deserialize_with = "string_or_event_target")]
  pub target: EventTarget,
  pub duration: f64,
  pub action: String,
}

fn string_or_event_target<'de, D>(deserializer: D) -> Result<EventTarget, D::Error>
  where D: Deserializer<'de>
{
  struct StringOrEventTarget(PhantomData<EventTarget>);

  impl<'de> Visitor<'de> for StringOrEventTarget {
    type Value = EventTarget;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
      formatter.write_str("\"global\", \"profile\", \"group\" or {\"group\":\"group_name\"}")
    }

    fn visit_str<E>(self, value: &str) -> Result<EventTarget, E>
      where E: de::Error
    {
      Ok(match value {
        "global" => EventTarget::Global,
        "profile" => EventTarget::Profile,
        "group" => EventTarget::Group(None),
        _ => return Err(de::Error::invalid_value(de::Unexpected::Str(value), &"global, group, or profile")),
      })
    }

    fn visit_map<M>(self, mut visitor: M) -> Result<EventTarget, M::Error>
      where M: MapAccess<'de>
    {
      let result = match visitor.next_key::<String>() {
        Ok(Some(ref key)) if key == "group" => {
          if let Ok(value) = visitor.next_value::<String>() {
            Ok(EventTarget::Group(Some(value)))
          } else {
            return Err(de::Error::invalid_type(de::Unexpected::Other("non-string"), &"string"))
          }
        }
        _ => return Err(de::Error::invalid_value(de::Unexpected::Other("anything that's not the string \"group\""), &"group")),
      };

      if let Ok(None) = visitor.next_key::<String>() {
        result
      } else {
        Err(de::Error::invalid_length(visitor.size_hint().unwrap_or(2), &"one key named \"group\""))
      }
    }
  }

  deserializer.deserialize_any(StringOrEventTarget(PhantomData))
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct JsonConfig {
  #[serde(default)]
  pub group_types: Vec<String>,
  #[serde(default)]
  pub collectables: HashMap<String, Collectable>,
  #[serde(default)]
  pub events: HashMap<String, Event>,
}

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
    match self {
      &JsonError::Serde(ref e) => e.description(),
      &JsonError::Io(ref e) => e.description(),
      &JsonError::Deserialize(ref s) => s.as_str(),
    }
  }
}

impl de::Error for JsonError {
  fn custom<T>(msg: T) -> Self where T : fmt::Display {
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

pub fn read_json_config(filename: &Path) -> Result<JsonConfig, JsonError> {
  Ok(serde_json::from_reader(File::open(filename)?)?)
}