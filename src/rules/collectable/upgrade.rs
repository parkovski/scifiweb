use super::Collectable;

pub struct Upgrade<'a> {
  pub owned_kind: *const Collectable<'a>,
  pub owned_amount: i32,
  pub upgrade_kind: *const Collectable<'a>,
  pub upgrade_applicable_level: i32,
}

pub struct UpgradeList<'a> {
  upgrades: Vec<Upgrade<'a>>,
}

impl<'a> UpgradeList<'a> {
  pub fn new() -> Self {
    UpgradeList {
      upgrades: Vec::new(),
    }
  }

  pub fn add(&'a mut self, upgrade: Upgrade<'a>) {
    self.upgrades.push(upgrade);
    self.upgrades.sort_by(|a, b| a.upgrade_applicable_level.cmp(&b.upgrade_applicable_level));
  }

  pub fn get_for_level(&'a self, level: i32) -> Option<&'a Upgrade<'a>> {
    self.upgrades
      .binary_search_by(|probe| probe.upgrade_applicable_level.cmp(&level))
      .ok()
      .map(|i| &self.upgrades[i])
  }
}