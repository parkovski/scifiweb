use std::sync::Arc;
use fxhash::FxHashMap;
use util::graph_cell::*;
use compile::{TokenSpan, TokenValue};
use ast::var::Variable;
use super::*;

/// Group membership default. An `Allow` group
/// can be joined by any type of player not in the
/// deny list. A `Deny` group can only be joined by those
/// types in the allow list.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize)]
pub enum MembershipMode {
  Allow,
  Deny,
}

impl Default for MembershipMode {
  fn default() -> Self {
    MembershipMode::Allow
  }
}

/// For groups that cannot be combined with other groups,
/// precedence defines the behavior when a new group is added.
/// If a group with higher precedence is already joined,
/// the new group will be disabled. If the new group has
/// higher precedence, the existing group will be disabled.
/// If the two have equal precedence, both will be disabled,
/// and if either has undefined precedence, the new group
/// will fail to add.
#[derive(Debug, Serialize)]
pub enum Precedence<'a> {
  Undefined,
  Higher(ItemRef<'a, UserGroup<'a>>),
  Equal(ItemRef<'a, UserGroup<'a>>),
  Lower(ItemRef<'a, UserGroup<'a>>),
}

impl<'a> Default for Precedence<'a> {
  fn default() -> Self {
    Precedence::Undefined
  }
}

/// This is a classification, and not a super type of User. A user
/// can belong to multiple user groups, which mostly just serve to
/// establish permissions.
#[derive(Debug, Serialize)]
pub struct UserGroup<'a> {
  name: TokenValue<Arc<str>>,
  /// Whether to allow or deny membership by default.
  membership_mode: MembershipMode,
  /// User types to allow to join this group when in `Deny` mode,
  /// or to deny when in `Allow` mode.
  except_members: Vec<ItemRef<'a, User<'a>>>,
  /// User groups that cannot be combined with this group.
  deny_with: Vec<ItemRef<'a, UserGroup<'a>>>,
  /// Group combination behavior. Cyclic precedence graphs are invalid.
  precedence: Precedence<'a>,
}

impl_named!(type UserGroup<'a>);
impl_name_traits!(UserGroup<'a>);
named_display!(UserGroup<'a>);

impl<'a> UserGroup<'a> {
  pub fn new(name: TokenValue<Arc<str>>) -> Self {
    UserGroup {
      name,
      membership_mode: Default::default(),
      except_members: Vec::new(),
      deny_with: Vec::new(),
      precedence: Default::default(),
    }
  }
}

impl<'a> SourceItem for UserGroup<'a> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> CastType<'a> for UserGroup<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::UserGroup;
}

impl<'a> CustomType<'a> for UserGroup<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::UserGroup
  }

  fn capabilities(&self) -> TypeCapability {
    TC_NOTIFY_RECEIVER
  }
}

/// This does not represent a single user. It is a user type that can belong
/// to any number of user groups, have properties, and has automatic
/// collectable ownership, notification target, and authentication functionality.
/// A game user is an instance of this type.
/// User types and user groups can be targeted by remote events, but
/// that is not specified here. Those targets are listed on the events,
/// which will be incorporated into the generated user types for the client.
#[derive(Debug, Serialize)]
pub struct User<'a> {
  name: TokenValue<Arc<str>>,
  properties: FxHashMap<Arc<str>, GraphCell<Variable<'a>>>,
}

impl_named!(type User<'a>);
impl_name_traits!(User<'a>);
named_display!(User<'a>);

impl<'a> User<'a> {
  pub fn new(name: TokenValue<Arc<str>>) -> Self {
    User { name, properties: Default::default() }
  }
}

impl<'a> SourceItem for User<'a> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    Ok(())
  }
}

impl<'a> CastType<'a> for User<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::User;
}

impl<'a> CustomType<'a> for User<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::User
  }

  fn capabilities(&self) -> TypeCapability {
    TC_NOTIFY_RECEIVER | TC_PROPERTIES
  }

  fn property(&self, name: &str) -> Option<GraphRef<'a, Variable<'a>>> {
    self.properties.get(name).map(|p| p.asleep())
  }
}
