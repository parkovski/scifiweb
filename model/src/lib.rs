#![allow(dead_code)]
#![cfg_attr(not(feature = "cargo-clippy"), allow(unknown_lints))]
#![feature(conservative_impl_trait, box_syntax)]

extern crate either;
extern crate futures;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate scifi_util as util;

pub mod access;
pub mod instance;
pub mod rules;

//pub mod event;

/// How long does the entity expect to
/// need to exist? These correlate with
/// whether it should be kept in cache
/// or moved to the database.
pub enum StoragePreference {
  /// Default - no hint.
  Unknown,
  /// The entity is too short-lived or
  /// unimportant to be placed in storage.
  NotStored,
  /// Lifetime measured in seconds.
  ShortTerm,
  /// Lifetime measured in minutes.
  MediumTerm,
  /// Lifetime from hours to permanent.
  LongTerm,
  /// Lots of frequent reads/writes/creates/deletes.
  HeavyTraffic,
}

/// IDs start at 1 - 0 represents an invalid or missing ID.
pub const ENTITY_INVALID_ID: u64 = 0;

/// All storable entities must be uniquely identifiable
/// by their type tag and ID.
pub trait Entity {
  /// This string must be unique to each type,
  /// and may not change once in use (permanent storage).
  const TYPE_TAG: &'static str;

  /// Which type of storage does the entity prefer?
  const STORAGE_PREFERENCE: StoragePreference = StoragePreference::Unknown;

  /// The ID must be unique within each type.
  fn id(&self) -> u64;
}

/// The object-safe version of Entity, auto-implemented
/// for anything that implements Entity.
pub trait EntityObject {
  /// Returns `Entity::TYPE_TAG`.
  fn type_tag(&self) -> &'static str;
  /// Returns `Entity::STORAGE_PREFERENCE`.
  fn storage_preference(&self) -> StoragePreference;
  /// Returns `Entity::id()`. Named differently to avoid conflicts.
  fn entity_id(&self) -> u64;
}

impl<T: Entity> EntityObject for T {
  fn type_tag(&self) -> &'static str {
    <Self as Entity>::TYPE_TAG
  }

  fn storage_preference(&self) -> StoragePreference {
    <Self as Entity>::STORAGE_PREFERENCE
  }

  fn entity_id(&self) -> u64 {
    <Self as Entity>::id(self)
  }
}

/// Add all entities to the event registries.
/// This must be done on startup, and the registry
/// will panic at runtime without them.
/// TODO: Create a script/build step/compiler plugin
/// that checks this.
pub fn initialize() {
  //event::registry::EventListenerRegistry::initialize();
  //event::registry::EventTriggerRegistry::initialize();
  trace!("Event registries initialized");
}
