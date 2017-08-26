pub(crate) mod registry;
pub mod trigger;

use std::borrow::Cow;
use std::sync::Arc;
use futures::stream::{Stream, futures_unordered};
use super::Entity;
use super::access::Accessor;
use self::trigger::EventTrigger;
use util::future::SFFuture;

pub struct Error;

type EventFuture<'a, T> = SFFuture<'a, T, Error>;

/// An event listener provides a way to save itself to
/// an accessor and restore itself when notified that
/// the event is completed. These must be registered at
/// startup or the registry will panic - see `registry.rs`.
pub trait EventListener {
  /// Retrieves self from the accessor.
  fn restore(id: u64, accessor: &Accessor<'static>) -> Box<EventListener>
  where Self: Entity + Sized;
  /// Notifies that the event completed.
  fn notify(&self) -> EventFuture<'static, ()>;
}

pub struct Event {
  trigger: Option<Box<EventTrigger>>,
  listeners: Arc<Vec<Box<EventListener>>>,
}

impl Event {
  pub fn new<T: EventTrigger + 'static>(trigger: T) -> Self {
    Event {
      trigger: Some(box trigger),
      listeners: Vec::new(),
    }
  }

  /// 
  pub fn preconstructed(
    trigger: Box<EventTrigger>,
    listeners: Vec<Box<EventListener>>,
  ) -> Self
  {
    Event {
      trigger: Some(trigger),
      listeners: Arc::new(listeners),
    }
  }

  pub fn add_listener<L: EventListener + 'static>(&mut self, listener: L) {
    Arc::make_mut(self.listeners).push(listener);
  }

  pub fn schedule(mut self, accessor: &Accessor<'static>) -> EventFuture<'static, ()> {
    let trigger = self.trigger.take().unwrap();
    trigger.schedule(self, accessor);
  }

  pub(crate) fn emit(&self) -> impl Stream<Item = (), Error = Error> + 'static {
    futures_unordered(
      self
        .listeners
        .clone()
        .iter()
        .map(|listener| listener.notify())
    )
  }
}

struct SerializedEvent {
  id: u64,
  trigger: (Cow<'static, str>, u64),
  listeners: Box<[(Cow<'static, str>, u64)]>,
}

impl Entity for SerializedEvent {
  const TYPE_TAG: &'static str = "scifi_model::event::SerializedEvent";

  fn id(&self) -> u64 {
    self.id
  }
}
