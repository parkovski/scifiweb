mod redemption;
mod upgrade;

use std::hash::{Hash, Hasher};
use either::Either;
use super::event::Event;

pub use self::redemption::{Redemption, RedemptionList};
pub use self::upgrade::{Upgrade, UpgradeList};

pub struct Collectable<'a> {
  pub name: String,
  redemptions: RedemptionList<'a>,
  upgrades: UpgradeList<'a>,
}

impl<'a> Collectable<'a> {
  pub fn new(name: String) -> Self {
    Collectable {
      name: name,
      redemptions: RedemptionList::new(),
      upgrades: UpgradeList::new(),
    }
  }

  pub fn add_collectable_redemption(
    &'a mut self,
    owned_amount: i32,
    owned_kind: *const Collectable<'a>,
    redeemable_amount: i32,
  ) {
    let redeemable_kind = self as *const _;
    self.redemptions.add(Redemption {
      owned_kind: Either::Left(owned_kind),
      owned_amount,
      redeemable_kind,
      redeemable_amount,
    });
  }

  pub fn add_event_redemption(&'a mut self, owned_kind: *const Event, redeemable_amount: i32) {
    let redeemable_kind = self as *const _;
    self.redemptions.add(Redemption {
      owned_kind: Either::Right(owned_kind),
      owned_amount: 1,
      redeemable_kind,
      redeemable_amount,
    });
  }

  pub fn add_upgrade(
    &'a mut self,
    owned_amount: i32,
    owned_kind: *const Collectable<'a>,
    upgrade_applicable_level: i32,
  ) {
    let upgrade_kind = self as *const _;
    self.upgrades.add(Upgrade {
      owned_kind,
      owned_amount,
      upgrade_kind,
      upgrade_applicable_level,
    });
  }
}

impl<'a> Hash for Collectable<'a> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    self.name.hash(state);
  }
}

impl<'a> PartialEq for Collectable<'a> {
  fn eq(&self, other: &Collectable<'a>) -> bool {
    self.name == other.name
  }
}

impl<'a> Eq for Collectable<'a> {}
