use super::collectable::{ CollectableInstanceList, CollectableInstance };
use auth::AccountManager;
use super::group::GroupMap;

pub struct Profile<'a> {
  pub name: String,
  pub level: CollectableInstance<'a>,
  pub accounts: AccountManager,
  pub groups: GroupMap<'a>,
  pub collectables: CollectableInstanceList<'a>,
}