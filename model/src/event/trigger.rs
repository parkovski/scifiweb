use std::time::Duration;
use sf_util::split_vec::SplitVec;
use sf_util::future::SFFuture;
//use super::collectable::Cost;
use super::{Event, EventFuture};
use access::Accessor as EventAccessor;
use ::{Entity, ENTITY_INVALID_ID};

/// TODO: Remove
pub struct Cost {_dummy: u32}

/// Represents a way for an event to notify its listeners.
/// Unfortunately, there is no good way to deserialize
/// unknown types, so each of these must define an associated function
/// `restore(serialized: &str) -> Box<EventTrigger>`
/// which is saved as a function pointer and uses the type tag to
/// decide which implementation to call.
pub trait EventTrigger {
  /// Activate the trigger. If the trigger is satisfied,
  /// the future will return true and the event will call
  /// its listeners.
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  /// Saves the event and trigger if necessary.
  fn schedule(&self, event: Event, accessor: &EventAccessor<'static>) -> EventFuture<'static, ()>;
}

/// The event fires as soon as it is scheduled.
pub struct AutomaticEventTrigger;

impl AutomaticEventTrigger {
  pub fn restore(_serialized: &str) -> Box<EventTrigger> {
    Box::new(AutomaticEventTrigger)
  }
}

impl Entity for AutomaticEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::AutomaticEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for AutomaticEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// The endpoint for this event was called. If `user_id`
/// is valid, the endpoint must be called by that user,
/// otherwise any user may call the endpoint.
pub struct UserEventTrigger {
  user_id: u64,
}

impl UserEventTrigger {
  pub fn new(user_id: u64) -> Self {
    UserEventTrigger { user_id }
  }

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(ENTITY_INVALID_ID))
  }
}

impl Entity for UserEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::UserEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for UserEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// This event is triggered when any user in the specified
/// `AuthenticationGroup` calls the event endpoint.
pub struct AuthorizedGroupEventTrigger {
  group_id: u64,
}

impl AuthorizedGroupEventTrigger {
  pub fn new(group_id: u64) -> Self {
    AuthorizedGroupEventTrigger { group_id }
  }

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(ENTITY_INVALID_ID))
  }
}

impl Entity for AuthorizedGroupEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::AuthorizedGroupEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for AuthorizedGroupEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
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

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(Duration::from_secs(0)))
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
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}
/// The event is triggered when a user agrees to pay the amount
/// of `Collectable` specified by the `Cost` parameter.
pub struct CostEventTrigger {
  cost: Cost,
}

impl CostEventTrigger {
  pub fn new(cost: Cost) -> Self {
    CostEventTrigger { cost }
  }

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(Cost {_dummy: 0}))
  }
}

impl Entity for CostEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::CostEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for CostEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// All of the triggers in the sequence must be satisfied in order.
pub struct SequenceEventTrigger {
  pending_index: usize,
  seq: Box<[Box<EventTrigger>]>,
}

impl SequenceEventTrigger {
  pub fn new<T: Into<Box<[Box<EventTrigger>]>>>(seq: T) -> Self {
    SequenceEventTrigger { pending_index: 0, seq: seq.into() }
  }

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(Vec::new()))
  }
}

impl Entity for SequenceEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::SequenceEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for SequenceEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// All of the triggers in the set must be satisfied, but in no
/// particular order. It is expected that the set is small since
/// the search algorithm is a linear scan.
pub struct SetEventTrigger {
  set: SplitVec<Box<EventTrigger>>,
}

impl SetEventTrigger {
  pub fn new<T: Into<Vec<Box<EventTrigger>>>>(set: T) -> Self {
    SetEventTrigger { set: SplitVec::left_from_vec(set.into()) }
  }

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(Vec::new()))
  }
}

impl Entity for SetEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::SetEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for SetEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
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
  count: u32,
  trigger: Box<EventTrigger>,
}

impl RepeatEventTrigger {
  pub fn new<T: EventTrigger + 'static>(count: u32, trigger: T) -> Self {
    RepeatEventTrigger { count, trigger: Box::new(trigger) }
  }

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(0, AutomaticEventTrigger))
  }
}

impl Entity for RepeatEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::RepeatEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for RepeatEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}

/// This event is satisfied when the linked event is satisfied.
pub struct LinkedEventTrigger {
  event_id: u64,
}

impl LinkedEventTrigger {
  pub fn new(event_id: u64) -> Self {
    LinkedEventTrigger { event_id }
  }

  pub fn restore(serialized: &str) -> Box<EventTrigger> {
    Box::new(Self::new(ENTITY_INVALID_ID))
  }
}

impl Entity for LinkedEventTrigger {
  const TYPE_TAG: &'static str = "sf_model::event::trigger::LinkedEventTrigger";

  fn id(&self) -> u64 {
    ENTITY_INVALID_ID
  }
}

impl EventTrigger for LinkedEventTrigger {
  fn trigger(&mut self) -> EventFuture<'static, bool> {
    SFFuture::new(Ok(true))
  }

  fn schedule(&self, _: Event, _: &EventAccessor<'static>) -> EventFuture<'static, ()> {
    SFFuture::new(Ok(()))
  }
}
