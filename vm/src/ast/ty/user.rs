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
pub enum Precedence<'ast> {
  Undefined,
  Higher(ItemRef<'ast, UserGroup<'ast>>),
  Equal(ItemRef<'ast, UserGroup<'ast>>),
  Lower(ItemRef<'ast, UserGroup<'ast>>),
}

impl<'ast> Default for Precedence<'ast> {
  fn default() -> Self {
    Precedence::Undefined
  }
}

/// This is a classification, and not a super type of User. A user
/// can belong to multiple user groups, which mostly just serve to
/// establish permissions.
#[derive(Debug, Serialize)]
pub struct UserGroup<'ast> {
  name: TokenValue<Arc<str>>,
  scope: GraphCell<Scope<'ast>>,
  /// Whether to allow or deny membership by default.
  membership_mode: MembershipMode,
  /// User types to allow to join this group when in `Deny` mode,
  /// or to deny when in `Allow` mode.
  except_members: Vec<ItemRef<'ast, User<'ast>>>,
  /// User groups that cannot be combined with this group.
  deny_with: Vec<ItemRef<'ast, UserGroup<'ast>>>,
  /// Group combination behavior. Cyclic precedence graphs are invalid.
  precedence: Precedence<'ast>,
}

type_macros!(
  UserGroup<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('ast,)
);

impl<'ast> UserGroup<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    Ast::insert_cast_type(
      ast,
      UserGroup {
        name,
        scope: Scope::child(parent_scope, ScopeKind::TYPE, span),
        membership_mode: Default::default(),
        except_members: Vec::new(),
        deny_with: Vec::new(),
        precedence: Default::default(),
      }
    )
  }
}

impl<'ast> SourceItem for UserGroup<'ast> {
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

impl<'ast> CastType<'ast> for UserGroup<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::UserGroup;
}

impl<'ast> CustomType<'ast> for UserGroup<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::UserGroup
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::NOTIFY_RECEIVER
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
pub struct User<'ast> {
  name: TokenValue<Arc<str>>,
  properties: FxHashMap<Arc<str>, GraphCell<Variable<'ast>>>,
  scope: GraphCell<Scope<'ast>>,
}

impl<'ast> User<'ast> {
  pub fn new(name: TokenValue<Arc<str>>, ast: GraphRefMut<'ast, Ast<'ast>>)
    -> Result<GraphRefMut<'ast, Self>>
  {
    let parent_scope = ast.awake().scope();
    let span = name.span().clone();
    Ast::insert_cast_type(
      ast,
      User {
        name,
        properties: Default::default(),
        scope: Scope::child(parent_scope, ScopeKind::TYPE, span),
      }
    )
  }
}

type_macros!(
  User<'ast>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('ast,)
);

impl<'ast> SourceItem for User<'ast> {
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

impl<'ast> CastType<'ast> for User<'ast> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::User;
}

impl<'ast> CustomType<'ast> for User<'ast> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::User
  }

  fn capabilities(&self) -> TypeCapability {
    TypeCapability::NOTIFY_RECEIVER | TypeCapability::PROPERTIES
  }

  fn property(&self, name: &str) -> Option<GraphRef<'ast, Variable<'ast>>> {
    self.properties.get(name).map(|p| p.asleep())
  }
}
