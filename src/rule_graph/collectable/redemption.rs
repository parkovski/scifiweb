use std::collections::HashMap;
use std::collections::hash_map::Entry;

use either::Either;

use super::Collectable;
use super::super::event::Event;

pub struct Redemption<'a> {
  pub owned_kind: Either<*const Collectable<'a>, *const Event<'a>>,
  pub owned_amount: i32,
  pub redeemable_kind: *const Collectable<'a>,
  pub redeemable_amount: i32,
}

pub struct RedemptionList<'a> {
  redemptions: HashMap<&'a Collectable<'a>, Vec<Redemption<'a>>>,
}

impl<'a> RedemptionList<'a> {
  pub fn new() -> Self {
    RedemptionList {
      redemptions: HashMap::new(),
    }
  }

  pub fn get(&'a self, kind: &'a Collectable<'a>) -> Option<&'a Vec<Redemption>> {
    self.redemptions.get(&kind)
  }

  pub fn add(&'a mut self, redemption: Redemption<'a>) {
    match self.redemptions.entry(unsafe { &*redemption.redeemable_kind }) {
      Entry::Occupied(mut e) => { e.get_mut().push(redemption); }
      Entry::Vacant(e) => { e.insert(vec![redemption]); }
    }
  }
}