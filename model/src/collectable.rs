use std::borrow::Cow;

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Cost {
  pub collectable_id: u64,
  pub amount: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct LevelRange {
  min_level: u32,
  max_level: u32,
}

/// If min > max, it indicates any
/// level. If min == max, it indicates
/// a single level.
impl LevelRange {
  pub fn min_level(&self) -> u32 {
    self.min_level
  }

  pub fn max_level(&self) -> u32 {
    self.max_level
  }

  pub fn is_single(&self) -> bool {
    self.min_level == self.max_level
  }

  pub fn is_any(&self) -> bool {
    self.min_level > self.max_level
  }

  pub fn any() -> LevelRange {
    LevelRange {
      min_level: 1,
      max_level: 0,
    }
  }
}

impl PartialEq for LevelRange {
  fn eq(&self, other: &LevelRange) -> bool {
    if (self.min_level > self.max_level && other.min_level > other.max_level) {
      true
    } else {
      self.min_level == other.min_level && self.max_level == other.max_level
    }
  }
}

impl Eq for LevelRange {}

#[derive(Debug, PartialEq, Eq)]
pub struct Upgrade {
  costs: Box<[Cost]>,
  applicable_levels: LevelRange,
  levels_to_add: i32,
}

impl Upgrade {
  pub fn new<'a>(
    costs: Cow<'a, [Cost]>,
    applicable_levels: LevelRange,
    levels_to_add: u32
  ) -> Self
  {
    Upgrade {
      costs: costs.into_owned().into_boxed_slice(),
      applicable_levels,
      levels_to_add
    }
  }

  pub fn costs(&self) -> &[Cost] {
    &self.costs
  }

  pub fn applicable_levels(&self) -> LevelRange {
    self.applicable_levels
  }

  pub fn levels_to_add(&self) -> i32 {
    self.levels_to_add
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Redemption {
  costs: Box<[Cost]>,
  award_amount: u32,
}