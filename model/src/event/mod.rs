pub(crate) mod registry;
pub mod trigger;

use std::borrow::Cow;
use super::Entity;
use super::access::Accessor;
use self::trigger::EventTrigger;
use sf_util::future::SFFuture;

pub struct Error;

type EventFuture<'a, T> = SFFuture<'a, T, Error>;

/// An event listener provides a way to save itself to
/// an accessor and restore itself when notified that
/// the event is completed. These must be registered at
/// startup or the registry will panic - see `registry.rs`.
pub trait EventListener {
  /// Retrieves self from the accessor and returns a future
  /// that can execute an action on event completion.
  fn restore_notify(accessor: &Accessor<'static>, id: u64) -> EventFuture<'static, ()>
  where Self: Sized;

  fn notify(&self, accessor: &Accessor<'static>) -> EventFuture<'static, ()>;
}

pub struct Event {
  trigger: Option<Box<EventTrigger>>,
  listeners: Vec<Box<EventListener>>,
}

impl Event {
  pub fn new<T: EventTrigger + 'static>(trigger: T) -> Self {
    Event {
      trigger: Some(Box::new(trigger)),
      listeners: Vec::new(),
    }
  }

  /// 
  pub fn preconstructed(
    trigger: Box<EventTrigger>,
    listeners: Vec<Box<EventListener>>
  ) -> Self
  {
    Event {
      trigger: Some(trigger),
      listeners,
    }
  }

  pub fn add_listener<L: EventListener + 'static>(&mut self, listener: L) {
    self.listeners.push(Box::new(listener));
  }

  pub fn schedule(mut self, accessor: &Accessor<'static>) -> EventFuture<'static, ()> {
    let trigger = self.trigger.take().unwrap();
    trigger.schedule(self, accessor)
  }

  pub(crate) fn emit(&self) -> EventFuture<()> {
    SFFuture::new(Ok(()))
  }
}

struct SerializedEvent {
  id: u64,
  trigger: (Cow<'static, str>, u64),
  listeners: Box<[(Cow<'static, str>, u64)]>,
}

impl Entity for SerializedEvent {
  const TYPE_TAG: &'static str = "sf_model::event::SerializedEvent";

  fn id(&self) -> u64 {
    self.id
  }
}
