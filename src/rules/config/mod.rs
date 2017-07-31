use std::error::Error;
use std::time::Duration;
use std::fmt;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::iter::FromIterator;

use super::{collectable, event, group, RuleGraph};

pub mod json;
pub use self::json::{read_json_rules, JsonRules};

#[derive(Debug)]
pub struct JsonConvertError {
  description: String,
}

impl JsonConvertError {
  pub fn new(description: String) -> Self {
    JsonConvertError { description }
  }

  pub fn not_found(kind: &'static str, name: &String) -> Self {
    JsonConvertError {
      description: format!("{} {} not found", kind, name),
    }
  }

  pub fn already_processed(kind: &'static str) -> Self {
    JsonConvertError {
      description: format!("{} already processed, now missing", kind),
    }
  }

  pub fn duplicate(kind: &'static str, name: &String) -> Self {
    JsonConvertError {
      description: format!("duplicate {} found: {}", kind, name),
    }
  }
}

impl fmt::Display for JsonConvertError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    write!(f, "{}", self.description.as_str())
  }
}

impl Error for JsonConvertError {
  fn description(&self) -> &str {
    self.description.as_str()
  }
}

pub struct JsonToGraphConverter<'a> {
  json_config: json::JsonRules,
  group_type_map: Option<HashMap<String, group::GroupType>>,
  event_map: Option<HashMap<String, event::Event>>,
  collectable_list: Option<Vec<collectable::Collectable<'a>>>,
  collectable_map: HashMap<
    String,
    (
      *mut collectable::Collectable<'a>,
      Vec<json::Redemption>,
      Vec<json::Upgrade>,
    ),
  >,
}

impl<'a> JsonToGraphConverter<'a> {
  pub fn new(json_config: json::JsonRules) -> Self {
    JsonToGraphConverter {
      json_config,
      group_type_map: Some(HashMap::new()),
      event_map: Some(HashMap::new()),
      collectable_list: Some(Vec::new()),
      collectable_map: HashMap::new(),
    }
  }

  pub fn convert(mut self) -> Result<RuleGraph<'a>, JsonConvertError> {
    self.convert_group_types()?;
    self.convert_events()?;
    self.convert_collectables()?;
    // Each of the above will return an error if their Option value is None.
    self.collectable_map.clear();
    let result = (
      self.group_type_map.take().unwrap(),
      HashMap::from_iter(
        self
          .collectable_list
          .take()
          .unwrap()
          .into_iter()
          .map(|c| (c.name.clone(), c)),
      ),
      self.event_map.take().unwrap(),
    );
    Ok(RuleGraph::new(result.0, result.1, result.2))
  }

  fn convert_group_types(&mut self) -> Result<(), JsonConvertError> {
    if let Some(ref mut group_type_map) = self.group_type_map {
      for json_group_type in self.json_config.group_types.drain(..) {
        match group_type_map.entry(json_group_type.clone()) {
          Entry::Occupied(_) => {
            return Err(JsonConvertError::duplicate("group type", &json_group_type))
          }
          Entry::Vacant(e) => {
            e.insert(group::GroupType::new(json_group_type));
          }
        }
      }
      Ok(())
    } else {
      Err(JsonConvertError::already_processed("group types"))
    }
  }

  fn convert_collectables(&mut self) -> Result<(), JsonConvertError> {
    {
      let collectable_list = self
        .collectable_list
        .as_mut()
        .ok_or_else(|| JsonConvertError::already_processed("collectables"))?;
      let collectable_map = &mut self.collectable_map;
      for json_collectable in self.json_config.collectables.drain() {
        collectable_list.push(collectable::Collectable::new(json_collectable.0.clone()));
        let collectable_ptr = {
          let index = collectable_list.len() - 1;
          let ptr = collectable_list.get_mut(index).unwrap() as *mut _;
          ptr
        };
        collectable_map.insert(
          json_collectable.0,
          (
            collectable_ptr,
            json_collectable.1.redemptions,
            json_collectable.1.upgrades,
          ),
        );
      }
    }
    for collectable in self.collectable_map.iter() {
      self
        .add_redemptions_and_upgrades((collectable.1).0, &(collectable.1).1, &(collectable.1).2)?;
    }
    Ok(())
  }

  fn add_redemptions_and_upgrades<'b>(
    &'b self,
    collectable: *mut collectable::Collectable<'a>,
    redemptions: &'b Vec<json::Redemption>,
    upgrades: &'b Vec<json::Upgrade>,
  ) -> Result<(), JsonConvertError> {
    let collectable_map = &self.collectable_map;
    let event_map = self.event_map.as_ref().unwrap();
    for r in redemptions.iter() {
      match r {
        &json::Redemption::Event {
          amount,
          cost_event: ref cost_event_name,
        } => if let Some(cost_event) = event_map.get(cost_event_name) {
          unsafe { (*collectable).add_event_redemption(cost_event as *const _, amount) }
        } else {
          return Err(JsonConvertError::not_found("event", cost_event_name));
        },
        &json::Redemption::Collectable {
          amount,
          cost_collectable: ref cost_collectable_name,
          cost_amount,
        } => if let Some(&(cost_collectable, _, _)) = collectable_map.get(cost_collectable_name) {
          unsafe {
            (*collectable)
              .add_collectable_redemption(cost_amount, cost_collectable as *const _, amount);
          }
        } else {
          return Err(JsonConvertError::not_found(
            "collectable",
            cost_collectable_name,
          ));
        },
      }
    }
    for u in upgrades.iter() {
      if let Some(&(cost_collectable, _, _)) = collectable_map.get(&u.cost_collectable) {
        unsafe {
          (*collectable).add_upgrade(u.cost_amount, cost_collectable as *const _, u.level);
        }
      } else {
        return Err(JsonConvertError::not_found(
          "collectable",
          &u.cost_collectable,
        ));
      }
    }
    Ok(())
  }

  fn convert_events(&mut self) -> Result<(), JsonConvertError> {
    let group_type_map = match self.group_type_map.as_ref() {
      Some(m) => m,
      None => return Err(JsonConvertError::already_processed("group types")),
    };
    let event_map = match self.event_map.as_mut() {
      Some(m) => m,
      None => return Err(JsonConvertError::already_processed("events")),
    };
    for json_event in self.json_config.events.drain() {
      // TODO: Check for errors.
      let event = event::Event {
        name: json_event.0,
        target: Self::convert_event_target(&json_event.1.target, group_type_map)?,
        duration: Duration::from_secs(json_event.1.duration as u64),
        action: event::Action::None,
      };
      event_map.insert(event.name.clone(), event);
    }
    Ok(())
  }

  fn convert_event_target(
    json_event_target: &json::EventTarget,
    group_type_map: &HashMap<String, group::GroupType>,
  ) -> Result<event::EventTarget, JsonConvertError> {
    match json_event_target {
      &json::EventTarget::Global => Ok(event::EventTarget::Global),
      &json::EventTarget::Profile => Ok(event::EventTarget::Profile),
      &json::EventTarget::GroupType(None) => Ok(event::EventTarget::Group),
      &json::EventTarget::GroupType(Some(ref group_type_name)) => {
        if let Some(group_type) = group_type_map.get(group_type_name) {
          Ok(event::EventTarget::GroupType(group_type as *const _))
        } else {
          Err(JsonConvertError::not_found("group type", group_type_name))
        }
      }
    }
  }
}
