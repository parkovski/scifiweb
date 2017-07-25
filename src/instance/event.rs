use std::time::Instant;

use rules;

use super::Target;

pub struct Event<'a> {
  rules: &'a rules::Event,
  start_time: Instant,
  target: Target,
}