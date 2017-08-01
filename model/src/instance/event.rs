use std::time::Instant;
use rules;
use super::Target;

pub struct Event<'a> {
  rules: &'a rules::event::Event,
  start_time: Instant,
  target: Target,
}
