use std::time::Duration;
use sf_util::split_vec::SplitVec;
use sf_util::future::SFFuture;
//use super::collectable::Cost;
use super::{Event, EventFuture, Error};
use access::Accessor as EventAccessor;
use ::{Entity, ENTITY_INVALID_ID, StoragePreference};

/// TODO: Remove
pub struct Cost {_dummy: u32}

/// Represents a way for an event to notify its listeners.
/// Event triggers define their own activation and storage
/// logic. The ID field for these types only matters post-
/// serialization, so `new()` will set an invalid ID - only
/// the accessor and `restore()` will set an ID.
pub trait EventTrigger {
  /// Activate the trigger. If the trigger is satisfied,
  /// the future will return true and the event will call
  /// its listeners.
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  /// Restore self from the registry. Pointers to this
  /// function are saved at startup and looked up by type tag.
  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> where Self: Entity + Sized;

  /// Saves the event and trigger if necessary.
  fn schedule(&self, event: Event, accessor: &EventAccessor<'static>) -> EventFuture<'static, ()>;
}

/// For a deserialization error, instead of wrapping
/// everything in `Result` and `Option`, we create
/// one of these which will return an error in its
/// futures - since triggers are only created to
/// call `trigger()` right away, we will see the
/// error as soon as possible anyway.
pub struct InvalidEventTrigger {
  reason: String,
}

impl InvalidEventTrigger {
  pub fn new<S: ToString>(reason: S) -> Self {
    InvalidEventTrigger { reason: reason.to_string() }
  }
}

impl Entity for InvalidEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::InvalidEventTrigger";

  const STORAGE_PREFERENCE: StoragePreference = StoragePreference::NotStored;

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for InvalidEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::err(Error)
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    box Self::new(format!("Tried to restore InvalidEventTrigger from '{}'", serialized))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::ok(())
  }
}

/// The event fires as soon as it is scheduled.
pub struct AutomaticEventTrigger(u64);

impl AutomaticEventTrigger {
  pub fn new() -> Self {
    AutomaticEventTrigger(ENTITY_INVALID_ID)
  }
}

impl Entity for AutomaticEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::AutomaticEventTrigger";

  const STORAGE_PREFERENCE: StoragePreference = StoragePreference::NotStored;

  fn id(&self) -> u64 {
    self.0
  }
}

impl EventTrigger for AutomaticEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn restore(id: u64, _serialized: &str) -> Box<EventTrigger> {
    Box::new(AutomaticEventTrigger)
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// The endpoint for this event was called. If `user_id`
/// is valid, the endpoint must be called by that user,
/// otherwise any user may call the endpoint.
pub struct UserEventTrigger {
  entity_id: u64,
  user_id: u64,
}

impl UserEventTrigger {
  pub fn new(user_id: u64) -> Self {
    UserEventTrigger { entity_id: ENTITY_INVALID_ID, user_id }
  }
}

impl Entity for UserEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::UserEventTrigger";

  fn id(&self) -> u64 {
    self.entity_id
  }
}

impl EventTrigger for UserEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(ENTITY_INVALID_ID))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// This event is triggered when any user in the specified
/// `AuthenticationGroup` calls the event endpoint.
pub struct AuthorizedGroupEventTrigger {
  entity_id: u64,
  group_id: u64,
}

impl AuthorizedGroupEventTrigger {
  pub fn new(group_id: u64) -> Self {
    AuthorizedGroupEventTrigger { entity_id: ENTITY_INVALID_ID, group_id }
  }
}

impl Entity for AuthorizedGroupEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::AuthorizedGroupEventTrigger";

  fn id(&self) -> u64 {
    self.entity_id
  }
}

impl EventTrigger for AuthorizedGroupEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(ENTITY_INVALID_ID))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// The event is triggered when the timer elapsed. The timer starts
/// as soon as the event is created.
pub struct TimerEventTrigger {
  duration: Duration,
}

impl TimerEventTrigger {
  pub fn new(duration: Duration) -> Self {
    TimerEventTrigger { duration }
  }
}

impl Entity for TimerEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::TimerEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for TimerEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::ok(true)
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(Duration::from_secs(0)))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::ok(())
  }
}
/// The event is triggered when a user agrees to pay the amount
/// of `Collectable` specified by the `Cost` parameter.
pub struct CostEventTrigger {
  id: u64,
  cost: Cost,
}

impl CostEventTrigger {
  pub fn new(cost: Cost) -> Self {
    CostEventTrigger { id: ENTITY_INVALID_ID, cost }
  }
}

impl Entity for CostEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::CostEventTrigger";

  fn id(&self) -> u64 {
    self.id
  }
}

impl EventTrigger for CostEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::ok(true)
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    box Self::new(Cost {_dummy: 0})
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::ok(())
  }
}

/// All of the triggers in the sequence must be satisfied in order.
pub struct SequenceEventTrigger {
  id: u64,
  pending_index: usize,
  seq: Box<[Box<EventTrigger>]>,
}

impl SequenceEventTrigger {
  pub fn new<T: Into<Box<[Box<EventTrigger>]>>>(seq: T) -> Self {
    SequenceEventTrigger { id: ENTITY_INVALID_ID, pending_index: 0, seq: seq.into() }
  }
}

impl Entity for SequenceEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::SequenceEventTrigger";

  fn id(&self) -> u64 {
    self.id
  }
}

impl EventTrigger for SequenceEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::ok(true)
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    box Self::new(Vec::new())
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::ok(())
  }
}

/// All of the triggers in the set must be satisfied, but in no
/// particular order. It is expected that the set is small since
/// the search algorithm is a linear scan.
pub struct SetEventTrigger {
  id: u64,
  set: SplitVec<Box<EventTrigger>>,
}

impl SetEventTrigger {
  pub fn new<T: Into<Vec<Box<EventTrigger>>>>(set: T) -> Self {
    SetEventTrigger { id: ENTITY_INVALID_ID, set: SplitVec::left_from_vec(set.into()) }
  }
}

impl Entity for SetEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::SetEventTrigger";

  fn id(&self) -> u64 {
    self.id
  }
}

impl EventTrigger for SetEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::ok(true)
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    box Self::new(Vec::new())
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::ok(())
  }
}

/*
/// Only one of the triggers in the option set must be satisfied.
pub struct OptionSetEventTrigger {
  set: [Box<EventTrigger>],
}

impl OptionSetEventTrigger

impl Entity for OptionSetEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::OptionSetEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}
*/

/// This event is completed when the inner trigger is called `count` times.
pub struct RepeatEventTrigger {
  id: u64,
  count: u32,
  trigger: Box<EventTrigger>,
}

impl RepeatEventTrigger {
  pub fn new<T: EventTrigger + 'static>(count: u32, trigger: T) -> Self {
    RepeatEventTrigger { id: ENTITY_INVALID_ID, count, trigger: box trigger }
  }
}

impl Entity for RepeatEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::RepeatEventTrigger";

  fn id(&self) -> u64 {
    self.id
  }
}

impl EventTrigger for RepeatEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::ok(true)
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    box Self::new(0, AutomaticEventTrigger)
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::ok(())
  }
}

/// This event is satisfied when the linked event is satisfied.
pub struct LinkedEventTrigger {
  entity_id: u64,
  event_id: u64,
}

impl LinkedEventTrigger {
  pub fn new(event_id: u64) -> Self {
    LinkedEventTrigger { entity_id: ENTITY_INVALID_ID, event_id }
  }
}

impl Entity for LinkedEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::LinkedEventTrigger";

  fn id(&self) -> u64 {
    self.id
  }
}

impl EventTrigger for LinkedEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::ok(true)
  }

  fn restore(id: u64, serialized: &str) -> Box<EventTrigger> {
    box Self::new(ENTITY_INVALID_ID)
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::ok(())
  }
}
