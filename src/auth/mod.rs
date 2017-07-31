mod admin;
pub use self::admin::AdminAuth;
pub use self::admin::GameServerAuth;

mod gamecenter;
pub use self::gamecenter::GameCenterAuth;

mod facebook;
pub use self::facebook::FacebookAuth;

mod googleplay;
pub use self::googleplay::GooglePlayAuth;

pub struct AccountManager {
  pub facebook_id: Option<u64>,
}
