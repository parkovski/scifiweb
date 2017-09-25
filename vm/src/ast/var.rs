use std::sync::Arc;
use std::fmt::{self, Display};
use fxhash::FxHashMap;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use compile::{TokenSpan, TokenValue};
use util::InsertGraphCell;
use util::graph_cell::*;
use ast::expr::BoxExpression;
use super::*;
use super::errors::*;
use super::ty::*;

#[derive(Debug, Serialize)]
pub struct Variable<'a> {
  name: TokenValue<Arc<str>>,
  ty: ItemRef<'a, Type<'a>>,
  initial: Option<BoxExpression<'a>>,
}

impl<'a> Variable<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    ty: ItemRef<'a, Type<'a>>,
  ) -> Self
  {
    Variable { name, ty, initial: None }
  }

  /// Only valid after resolve phase has succeeded.
  pub fn ty(&self) -> GraphRef<'a, Type<'a>> {
    self.ty.item().unwrap()
  }

  pub fn set_initial(&mut self, initial: BoxExpression<'a>) {
    self.initial = Some(initial);
  }
}

impl_named!("variable", Variable<'a>);
named_display!(Variable<'a>);

impl<'a> SourceItem for Variable<'a> {
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

/// Sets the initial value of an inherited variable.
#[derive(Debug, Serialize)]
pub struct DefaultValue<'a> {
  name: TokenValue<Arc<str>>,
  value: BoxExpression<'a>,
  #[serde(skip)]
  var: ResolveLater<'a, Variable<'a>, FilteredScope<'a>>,
}

impl<'a> DefaultValue<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    scope: GraphRef<'a, Scope<'a>>,
    value: BoxExpression<'a>,
  ) -> Self
  {
    let scope_range = scope.awake().kind().only();
    let filtered_scope = FilteredScope::new(scope, scope_range, true);
    DefaultValue {
      name: name.clone(),
      value,
      var: ResolveLater::Unresolved(filtered_scope),
    }
  }
}

impl_named!("default", DefaultValue<'a>);
named_display!(DefaultValue<'a>);

impl<'a> SourceItem for DefaultValue<'a> {
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

#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq)]
pub struct ScopeKindRange(ScopeKind, ScopeKind);

impl ScopeKindRange {
  fn contains(&self, kind: ScopeKind) -> bool {
    kind >= self.0 && kind <= self.1
  }
}

/// These are in order from least specific to most specific.
/// Searches need to be able to specify their specificity
/// when searching recursively through scopes.
#[derive(Debug, Serialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScopeKind {
  Global,
  Type,
  FnParam,
  FnLocal,
}

impl ScopeKind {
  fn all() -> ScopeKindRange {
    ScopeKindRange(ScopeKind::Global, ScopeKind::FnLocal)
  }

  fn only(self) -> ScopeKindRange {
    ScopeKindRange(self, self)
  }

  fn and_below(self) -> ScopeKindRange {
    ScopeKindRange(ScopeKind::Global, self)
  }
}

impl Display for ScopeKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    f.write_str(match *self {
      ScopeKind::FnLocal => "function local",
      ScopeKind::FnParam => "function param",
      ScopeKind::Type => "type",
      ScopeKind::Global => "global",
    })
  }
}

#[derive(Debug)]
pub struct Scope<'a> {
  kind: ScopeKind,
  vars: FxHashMap<Arc<str>, GraphCell<Variable<'a>>>,
  parent: Option<GraphRef<'a, Scope<'a>>>,
}

impl<'a> Scope<'a> {
  pub fn new() -> GraphCell<Self> {
    GraphCell::new(Scope {
      kind: ScopeKind::Global,
      vars: Default::default(),
      parent: None,
    })
  }

  pub fn child(this: GraphRef<'a, Scope<'a>>) -> GraphCell<Self> {
    GraphCell::new(Scope {
      kind: ScopeKind::Type,
      vars: Default::default(),
      parent: Some(this),
    })
  }

  pub fn parent(&self) -> Option<GraphRef<'a, Scope<'a>>> {
    self.parent
  }

  pub fn set_parent(&mut self, parent: GraphRef<'a, Scope<'a>>) -> Result<()> {
    if self.parent.is_some() {
      return Err(ErrorKind::InvalidOperation(
        "can't set parent on scope that already has a parent"
      ).into());
    }
    let p = parent.awake();
    for (key, value) in &self.vars {
      if p.has(key) {
        return Err(ErrorKind::DuplicateDefinition(
          value.awake().name().clone(), "variable"
        ).into());
      }
    }
    Ok(self.parent = Some(parent))
  }

  pub fn has(&self, name: &str) -> bool {
    self.vars.contains_key(name) || self.parent.map_or(false, |p| p.awake().has(name))
  }

  pub fn has_filtered(&self, name: &str, range: ScopeKindRange, recursive: bool) -> bool {
    if range.contains(self.kind) && self.vars.contains_key(name) {
      true
    } else if recursive {
      self.parent.map_or(false, |p| p.awake().has_filtered(name, range, true))
    } else {
      false
    }
  }

  pub fn find_filtered_mut(&self, name: &str, range: ScopeKindRange, recursive: bool)
    -> Option<GraphRefMut<'a, Variable<'a>>>
  {
    if range.contains(self.kind) {
      if let Some(v) = self.vars.get(name) {
        return Some(v.asleep_mut());
      }
    }
    if recursive {
      self.parent
        .map(|p| p.awake().find_filtered_mut(name, range, true))
        .unwrap_or(None)
    } else {
      None
    }
  }

  pub fn find_filtered(&self, name: &str, range: ScopeKindRange, recursive: bool)
    -> Option<GraphRef<'a, Variable<'a>>>
  {
    self.find_filtered_mut(name, range, recursive).map(|v| v.asleep_ref())
  }

  pub fn insert(&mut self, var: Variable<'a>) -> Result<GraphRefMut<'a, Variable<'a>>> {
    let error: Error = ErrorKind::DuplicateDefinition(
        var.name().clone(), "variable"
    ).into();
    if let Some(parent) = self.parent {
      if parent.awake().has(&var.name()) {
        return Err(error);
      }
    }
    self.vars
      .insert_graph_cell(var.name().value().clone(), var)
      .map_err(move |_| error)
  }

  fn kind(&self) -> ScopeKind {
    self.kind
  }

  fn level(&self) -> u32 {
    self.parent.map(|p| 1 + p.awake().level()).unwrap_or(0)
  }
}

impl<'a> Owner<'a, Variable<'a>> for Scope<'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Variable<'a>>> {
    self.vars.get(name)
      .map(|v| v.asleep_mut())
      .or_else(||
        self.parent.map(|p|
          p.awake().find_mut(name)
        )
        .unwrap_or(None)
      )
  }
}

impl<'a> Serialize for Scope<'a> {
  fn serialize<S: Serializer>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_struct("Scope", 2)?;
    state.serialize_field("level", &self.level())?;
    state.serialize_field("vars", &self.vars)?;
    state.end()
  }
}

pub trait Scoped<'a> {
  fn scope(&self) -> GraphRef<'a, Scope<'a>>;
  fn scope_mut(&mut self) -> GraphRefMut<'a, Scope<'a>>;
}

#[derive(Debug, Serialize)]
pub struct FilteredScope<'a> {
  scope: GraphRef<'a, Scope<'a>>,
  range: ScopeKindRange,
  recursive: bool,
}

impl<'a> FilteredScope<'a> {
  pub fn new(
    scope: GraphRef<'a, Scope<'a>>,
    range: ScopeKindRange,
    recursive: bool,
  ) -> Self
  {
    FilteredScope { scope, range, recursive }
  }

  pub fn to_inner(&self) -> GraphRef<'a, Scope<'a>> {
    self.scope
  }
}

impl<'a> Owner<'a, Variable<'a>> for FilteredScope<'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Variable<'a>>> {
    self.scope.awake().find_filtered_mut(name, self.range, self.recursive)
  }
}
