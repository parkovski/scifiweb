use std::sync::Arc;
use util::graph_cell::{GraphCell, GraphRef};
use compile::{TokenSpan, TokenValue};
use ast::{SourceItem, Named};
use ast::var::{Scope, ScopeKind};
use ast::ty::TypeCapability;
use ast::errors::*;
use super::{BaseCustomType, CustomType, CastType};

#[derive(Debug, Serialize)]
pub struct EarlyRefType<'a> {
  name: TokenValue<Arc<str>>,
  real_ty: BaseCustomType,
  parent: Option<GraphRef<'a, CustomType<'a>>>,
  /// This will never contain anything but if the user
  /// forgot to define this type there might be lookups
  /// against it before we get around to "resolving" this type.
  scope: GraphCell<Scope<'a>>,
}

impl<'a> EarlyRefType<'a> {
  pub fn new(name: TokenValue<Arc<str>>, real_ty: BaseCustomType) -> Self {
    let span = name.span().clone();
    EarlyRefType {
      name,
      real_ty,
      parent: None,
      scope: Scope::new(ScopeKind::TYPE, span),
    }
  }

  pub fn real_ty(&self) -> BaseCustomType {
    self.real_ty
  }

  pub fn parent<P>(&self) -> Result<Option<GraphRef<'a, P>>>
  where P: CustomType<'a> + CastType<'a> + 'a
  {
    let parent = match self.parent {
      Some(parent) => parent,
      None => return Ok(None),
    };
    let typed_parent = parent.map_opt(P::try_cast);
    if typed_parent.is_some() {
      Ok(typed_parent)
    } else {
      Err(ErrorKind::TypeResolution(
        Arc::from(<P as CastType>::BASE_TYPE.as_str()),
        parent.awake().name().clone()
      ).into())
    }
  }
}

type_macros!(
  EarlyRefType<'a>;

  impl_named(type),
  impl_name_traits,
  named_display,
  impl_scoped('a,)
);

impl<'a> SourceItem for EarlyRefType<'a> {
  fn span(&self) -> &TokenSpan {
    self.name.span()
  }

  fn resolve(&mut self) -> Result<()> {
    Err(ErrorKind::NotDefined(self.name.clone(), self.real_ty.as_str()).into())
  }

  fn typecheck(&mut self) -> Result<()> {
    panic!("Shouldn't have made it past the resolve phase")
  }
}

impl<'a> CastType<'a> for EarlyRefType<'a> {
  const BASE_TYPE: BaseCustomType = BaseCustomType::EarlyRef;
}

impl<'a> CustomType<'a> for EarlyRefType<'a> {
  fn base_type(&self) -> BaseCustomType {
    BaseCustomType::EarlyRef
  }

  fn capabilities(&self) -> TypeCapability {
    Default::default()
  }
}
