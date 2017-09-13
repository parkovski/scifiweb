use std::fmt;
use serde::de::{self, Deserialize, Deserializer, Visitor, SeqAccess, Unexpected};

#[derive(Debug)]
pub struct DebugOptions {
  pub save_ast: bool,
}

impl Default for DebugOptions {
  fn default() -> Self {
    DebugOptions {
      save_ast: false,
    }
  }
}

struct DebugOptionsVisitor {
  opts: DebugOptions,
}

impl DebugOptionsVisitor {
  pub fn new() -> Self {
    DebugOptionsVisitor { opts: Default::default() }
  }

  pub fn set_option(&mut self, option: &str) -> bool {
    if option == "save-ast" {
      self.opts.save_ast = true;
    } else {
      return false;
    }
    true
  }
}

impl<'de> Visitor<'de> for DebugOptionsVisitor {
  type Value = DebugOptions;

  fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str("a debug option string")
  }

  fn visit_str<E: de::Error>(mut self, v: &str) -> Result<Self::Value, E> {
    if self.set_option(v) {
      Ok(self.opts)
    } else {
      Err(E::invalid_value(Unexpected::Str(v), &self))
    }
  }

  fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<Self::Value, A::Error> {
    loop {
      match seq.next_element::<String>() {
        Ok(Some(opt)) => if !self.set_option(&opt) {
          return Err(de::Error::invalid_value(Unexpected::Str(&opt), &self));
        },
        Ok(None) => return Ok(self.opts),
        Err(e) => return Err(e),
      }
    }
  }
}

impl<'de> Deserialize<'de> for DebugOptions {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    deserializer.deserialize_seq(DebugOptionsVisitor::new())
  }
}
