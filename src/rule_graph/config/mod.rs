use std::error::Error;
use std::time::Duration;
use std::fmt;
use std::collections::HashMap;
use std::iter::FromIterator;

use super::{ group, collectable, event, RuleGraph };

pub mod json;
pub use self::json::{ read_json_config, JsonConfig, JsonError };

#[derive(Debug)]
pub struct JsonConvertError {
  description: String,
}

impl JsonConvertError {
  pub fn new(description: String) -> Self {
    JsonConvertError {
      description,
    }
  }

  pub fn new_not_found(kind: &'static str, name: &String) -> Self {
    JsonConvertError {
      description: format!("{} {} not found", kind, name),
    }
  }

  pub fn new_already_processed(kind: &'static str) -> Self {
    JsonConvertError {
      description: format!("{} already processed, now missing", kind),
    }
  }
}

impl fmt::Display for JsonConvertError {
  fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
    write!(f, "{}", self.description)
  }
}

impl Error for JsonConvertError {
  fn description(&self) -> &str {
    self.description.as_str()
  }
}

impl From<group::GroupAddError> for JsonConvertError {
  fn from(error: group::GroupAddError) -> Self {
    JsonConvertError::new(error.description().to_string())
  }
}

pub struct JsonToGraphConverter<'a> {
  json_config: json::JsonConfig,
  event_map: Option<HashMap<String, event::Event<'a>>>,
  group_map: Option<group::GroupMap<'a>>,
  collectable_list: Option<Vec<collectable::Collectable<'a>>>,
  collectable_map: HashMap<
    String, (
      *mut collectable::Collectable<'a>,
      Vec<json::Redemption>,
      Vec<json::Upgrade>
    )
  >,
}

impl<'a> JsonToGraphConverter<'a> {
  pub fn new(json_config: json::JsonConfig) -> Self {
    JsonToGraphConverter {
      json_config,
      event_map: Some(HashMap::new()),
      group_map: Some(group::GroupMap::new()),
      collectable_list: Some(Vec::new()),
      collectable_map: HashMap::new(),
    }
  }

  pub fn convert(&mut self) -> Result<RuleGraph<'a>, JsonConvertError> {
    self.convert_events()?;
    self.convert_groups()?;
    self.convert_collectables()?;
    // Each of the above will return an error if their Option value is None.
    self.collectable_map.clear();
    let result = (
      self.group_map.take().unwrap(),
      HashMap::from_iter(self.collectable_list.take().unwrap().into_iter().map(|c| (c.name.clone(), c))),
      self.event_map.take().unwrap(),
    );
    Ok(RuleGraph::new(result.0, result.1, result.2))
  }

  fn convert_groups(&mut self) -> Result<(), JsonConvertError> {
    if let Some(ref mut group_map) = self.group_map {
      for group_type in self.json_config.group_types.drain(..) {
        group_map.add_group_type(group_type)?;
      }
      Ok(())
    } else {
      Err(JsonConvertError::new_already_processed("groups"))
    }
  }

  fn convert_collectables(&mut self) -> Result<(), JsonConvertError> {
    {
      let collectable_list = self.collectable_list.as_mut().ok_or_else(|| JsonConvertError::new_already_processed("collectables"))?;
      let collectable_map = &mut self.collectable_map;
      for json_collectable in self.json_config.collectables.drain() {
        let mut collectable = collectable::Collectable::new(json_collectable.0.clone());
        let collectable_ptr = {
          let ptr = (&mut collectable) as *mut _;
          ptr
        };
        collectable_list.push(collectable);
        collectable_map.insert(
          json_collectable.0,
          (
            collectable_ptr,
            json_collectable.1.redemptions,
            json_collectable.1.upgrades
          )
        );
      }
    }
    for collectable in self.collectable_map.iter() {
      self.add_redemptions_and_upgrades(
        (collectable.1).0,
        &(collectable.1).1,
        &(collectable.1).2
      )?;
    }
    Ok(())
  }

  fn add_redemptions_and_upgrades<'b>(
    &'b self,
    collectable: *mut collectable::Collectable<'a>,
    redemptions: &'b Vec<json::Redemption>,
    upgrades: &'b Vec<json::Upgrade>
  ) -> Result<(), JsonConvertError>
  {
    let collectable_map = &self.collectable_map;
    let event_map = self.event_map.as_ref().unwrap();
    for r in redemptions.iter() {
      match r {
        &json::Redemption::Event { amount, cost_event: ref cost_event_name } => {
          if let Some(cost_event) = event_map.get(cost_event_name) {
            unsafe {
              (*collectable).add_event_redemption(
                cost_event as *const _,
                amount,
              )
            }
          } else {
            return Err(JsonConvertError::new_not_found("event", cost_event_name));
          }
        }
        &json::Redemption::Collectable { amount, cost_collectable: ref cost_collectable_name, cost_amount } => {
          if let Some(&(cost_collectable, _, _)) = collectable_map.get(cost_collectable_name) {
            unsafe {
              (*collectable).add_collectable_redemption(
                cost_amount,
                cost_collectable as *const _,
                amount,
              );
            }
          } else {
            return Err(JsonConvertError::new_not_found("collectable", cost_collectable_name));
          }
        }
      }
    }
    for u in upgrades.iter() {
      if let Some(&(cost_collectable, _, _)) = collectable_map.get(&u.cost_collectable) {
        unsafe {
          (*collectable).add_upgrade(u.cost_amount, cost_collectable as *const _, u.level);
        }
      } else {
        return Err(JsonConvertError::new_not_found("collectable", &u.cost_collectable));
      }
    }
    Ok(())
  }

  fn convert_events(&mut self) -> Result<(), JsonConvertError> {
    let event_map = self.event_map.as_mut().ok_or_else(|| JsonConvertError::new_already_processed("events"))?;
    for json_event in self.json_config.events.drain() {
      // TODO: Check for errors.
      let event = event::Event {
        name: json_event.0,
        target: None,
        duration: Duration::from_secs(json_event.1.duration as u64),
        action: event::Action::None,
      };
      event_map.insert(event.name.clone(), event);
    }
    Ok(())
  }
}