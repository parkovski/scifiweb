mod admin;
mod gamecenter;
mod facebook;
mod googleplay;

use std::borrow::Cow;
pub use self::admin::AdminAuth;
pub use self::admin::GameServerAuth;
pub use self::gamecenter::GameCenterAuth;
pub use self::facebook::FacebookAuth;
pub use self::googleplay::GooglePlayAuth;

pub struct AuthenticationGroup {
  id: u64,
  name: String,
}

impl AuthenticationGroup {
  pub fn new<'a>(id: u64, name: Cow<'a, str>) -> Self {
    AuthenticationGroup { id, name: name.into_owned() }
  }

  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn system_group() -> AuthenticationGroup {
    Self::new(0, "system".into())
  }
}

pub struct User {
  id: u64,
  name: String,
  groups: Vec<u64>,
}

impl User {
  pub fn id(&self) -> u64 {
    self.id
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn groups(&self) -> &[u64] {
    &self.groups
  }
}

pub struct AccountManager {
  pub facebook_id: Option<u64>,
}
