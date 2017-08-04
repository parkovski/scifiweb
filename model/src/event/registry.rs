use std::sync::{Once, ONCE_INIT};
use std::collections::HashMap;
use sf_util::future::SFFuture;
use access::Accessor;
use super::EventFuture;

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

type ListenerMap = HashMap<&'static str, fn(&Accessor<'static>, u64) -> EventFuture<'static, ()>>;
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
        // This technically leaks, but it is supposed to last
        // the duration of the program anyway, and won't be
        // changed after startup.
        LISTENER_REGISTRY.map = Box::into_raw(Box::new(map));
      }
    });
  }

  pub fn get(type_tag: &'static str) -> fn(&Accessor<'static>, u64) -> EventFuture<'static, ()> {
    fn foo(a: &Accessor<'static>, b: u64) -> EventFuture<'static, ()> {
      SFFuture::new(Ok(()))
    }
    foo
  }

  fn fill_map(map: &mut ListenerMap) {

  }

  fn error(_: &Accessor<'static>, _: u64) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

type TriggerMap = HashMap<&'static str, fn(&Accessor<'static>, u64) -> EventFuture<'static, bool>>;
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

  pub fn get(type_tag: &'static str) -> fn(&Accessor<'static>, u64) -> EventFuture<'static, bool> {
    fn foo(a: &Accessor<'static>, b: u64) -> EventFuture<'static, bool> {
      SFFuture::new(Ok(true))
    }
    foo
  }

  fn fill_map(map: &mut TriggerMap) {

  }

  fn error(_: &Accessor<'static>, _: u64) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }
}