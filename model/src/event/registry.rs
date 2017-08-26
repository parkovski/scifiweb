use std::sync::{Once, ONCE_INIT};
use std::collections::HashMap;
use util::future::SFFuture;
use access::Accessor;
use super::{EventFuture, EventListener};
use super::trigger::*;
use ::Entity;

// There is no synchronization around these because
// they are initialized before any threads are started
// and only ever read after that. If they ever need
// to be changed at runtime, make sure to add
// synchronization first.
static mut LISTENER_REGISTRY: EventListenerRegistry = EventListenerRegistry {
  map: 0 as *const _,
};
static mut TRIGGER_REGISTRY: EventTriggerRegistry = EventTriggerRegistry {
  map: 0 as *const _,
};

type ListenerMap = HashMap<&'static str, fn(&Accessor<'static>, u64) -> Box<EventListener>>;
pub struct EventListenerRegistry {
  map: *const ListenerMap,
}
unsafe impl Sync for EventListenerRegistry {}

impl EventListenerRegistry {
  pub fn initialize() {
    static ONCE: Once = ONCE_INIT;
    ONCE.call_once(|| {
      let mut map = HashMap::new();
      Self::fill_map(&mut map);
      unsafe {
        // Probably better to just leak this - it needs to be
        // around for the duration of the program, and while
        // it's easy to make sure it's initialized before any
        // multithreaded access, it's harder to be sure no
        // thread accesses invalid memory after this is freed.
        LISTENER_REGISTRY.map = Box::into_raw(Box::new(map));
      }
    });
  }

  pub fn try_get(type_tag: &'static str)
    -> Option<fn(&Accessor<'static>, u64) -> Box<EventListener>>
  {
    unsafe {
      *(*LISTENER_REGISTRY.map)
        .get(type_tag)
    }
  }

  #[cfg(debug_assertions)]
  pub fn get(type_tag: &'static str) -> fn(&Accessor<'static>, u64) -> Box<EventListener> {
    Self::try_get(type_tag)
      .expect("All event listeners must be added to the registry on startup")
  }

  #[cfg(not(debug_assertions))]
  pub fn get(type_tag: &'static str) -> fn(&Accessor<'static>, u64) -> Box<EventListener> {
    Self::try_get(type_tag)
      .ok_or_else(|| InvalidEventTrigger::new(
        format!("EventListenerRegistry has no entry for '{}'", type_tag)
      ))
  }

  fn fill_map(map: &mut ListenerMap) {

  }
}

type TriggerMap = HashMap<&'static str, fn(u64, &str) -> Box<EventTrigger>>;
pub struct EventTriggerRegistry {
  map: *const TriggerMap,
}
unsafe impl Sync for EventTriggerRegistry {}

impl EventTriggerRegistry {
  pub fn initialize() {
    static ONCE: Once = ONCE_INIT;
    ONCE.call_once(|| {
      let mut map = HashMap::new();
      Self::fill_map(&mut map);
      unsafe {
        TRIGGER_REGISTRY.map = Box::into_raw(Box::new(map));
      }
    });
  }

  pub fn try_get(type_tag: &'static str) -> Option<fn(u64, &str) -> Box<EventTrigger>> {
    unsafe {
      *(*TRIGGER_REGISTRY.map)
        .get(type_tag)
    }
  }

  #[cfg(debug_assertions)]
  pub fn get(type_tag: &'static str) -> fn(&str) -> Box<EventTrigger> {
    Self::try_get(type_tag)
      .expect("All event triggers must be added to the registry on startup")
  }

  #[cfg(not(debug_assertions))]
  pub fn get(type_tag: &'static str) -> fn(&str) -> Box<EventTrigger> {
    Self::try_get(type_tag)
      .ok_or_else(|| InvalidEventTrigger::new(
        format!("EventTriggerRegistry has no entry for '{}'", type_tag)
      ))
  }

  fn fill_map(map: &mut TriggerMap) {
    fn insert<T: EventTrigger + Entity>(map: &mut TriggerMap) {
      map.insert(<T as Entity>::TYPE_TAG, <T as EventTrigger>::restore);
    }

    insert::<AutomaticEventTrigger>(map);
    insert::<UserEventTrigger>(map);
    insert::<AuthorizedGroupEventTrigger>(map);
    insert::<TimerEventTrigger>(map);
    insert::<CostEventTrigger>(map);
    insert::<SequenceEventTrigger>(map);
    insert::<SetEventTrigger>(map);
    insert::<RepeatEventTrigger>(map);
    insert::<LinkedEventTrigger>(map);
  }
}