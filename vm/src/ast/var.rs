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
    if let Some(ref mut init) = self.initial {
      init.resolve()?;
    }
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    if let Some(ref mut init) = self.initial {
      init.typecheck()?;
    }
    Ok(())
  }
}

/// Sets the initial value of an inherited variable.
#[derive(Debug, Serialize)]
pub struct DefaultValue<'a> {
  name: TokenValue<Arc<str>>,
  value: BoxExpression<'a>,
  #[serde(skip)]
  var: ResolveLater<'a, Variable<'a>, ScopeFilter<'a>>,
}

impl<'a> DefaultValue<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    scope: GraphRef<'a, Scope<'a>>,
    value: BoxExpression<'a>,
  ) -> Self
  {
    let mut scope_kind = scope.awake().kind();
    scope_kind.remove(ScopeKind::RECURSIVE);
    let filtered_scope = ScopeFilter::new(scope, scope_kind);
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
    self.var.resolve(Owner::find, &self.name)?;
    self.value.resolve()
  }

  fn typecheck(&mut self) -> Result<()> {
    self.value.typecheck()
  }
}

bitflags! {
  /// Flags for scope implementation and lookup. Some flags are only
  /// valid for implementation - see `LOOKUP_MASK`.
  pub struct ScopeKind: u16 {
    /// The scope is the root.
    const GLOBAL    = 0x1;
    /// The scope is for a type container.
    const TYPE      = 0x2;
    /// The scope is for function parameters.
    const FN_PARAM  = 0x4;
    /// The scope is for function local variables.
    const FN_LOCAL  = 0x8;
    /// The scope includes parent scopes.
    const RECURSIVE = 0x10;
    /// The scope has a 'this' variable.
    const INSTANCE  = 0x20;
    /// The scope has a 'remote' variable.
    const REMOTE    = 0x40;
    /// The flags that are valid for lookups.
    const LOOKUP_MASK = 0x1F;
  }
}

impl Display for ScopeKind {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut flags = *self;
    let mut first = true;
    f.write_str("[")?;
    {
      let mut add = |kind: ScopeKind, rep: &str| -> fmt::Result {
        if !flags.contains(kind) {
          return Ok(());
        }
        flags.remove(kind);

        if !first {
          f.write_str(", ")?;
        } else {
          first = false;
        }
        f.write_str(rep)
      };

      add(ScopeKind::GLOBAL, "Global")?;
      add(ScopeKind::TYPE, "Type")?;
      add(ScopeKind::FN_PARAM, "FnParam")?;
      add(ScopeKind::FN_LOCAL, "FnLocal")?;
      add(ScopeKind::RECURSIVE, "Recursive")?;
      add(ScopeKind::INSTANCE, "Instance")?;
      add(ScopeKind::REMOTE, "Remote")?;
    }
    f.write_str("]")?;
    debug_assert!(flags.is_empty(), "Not all cases covered");
    Ok(())
  }
}

impl Serialize for ScopeKind {
  fn serialize<S: Serializer>(&self, serializer: S)
    -> ::std::result::Result<S::Ok, S::Error>
  {
    serializer.serialize_str(&self.to_string())
  }
}

#[derive(Debug)]
pub struct Scope<'a> {
  kind: ScopeKind,
  vars: FxHashMap<Arc<str>, GraphCell<Variable<'a>>>,
  parent: Option<GraphRef<'a, Scope<'a>>>,
  span: TokenSpan,
}

impl<'a> Scope<'a> {
  pub fn new(kind: ScopeKind, span: TokenSpan) -> GraphCell<Self> {
    GraphCell::new(Scope {
      kind,
      vars: Default::default(),
      parent: None,
      span,
    })
  }

  pub fn child(
    this: GraphRef<'a, Scope<'a>>,
    kind: ScopeKind, span: TokenSpan,
  ) -> GraphCell<Self>
  {
    GraphCell::new(Scope {
      kind,
      vars: Default::default(),
      parent: Some(this),
      span,
    })
  }

  /// We don't generally know how long a scope lasts until it ends,
  /// where this should be modified.
  pub fn span_mut(&mut self) -> &mut TokenSpan {
    &mut self.span
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

  pub fn has_filtered(&self, name: &str, kind: ScopeKind) -> bool {
    if kind.contains(self.kind) && self.vars.contains_key(name) {
      true
    } else if kind.contains(ScopeKind::RECURSIVE) {
      self.parent.map_or(false, |p| p.awake().has_filtered(name, kind))
    } else {
      false
    }
  }

  pub fn find_filtered_mut(&self, name: &str, kind: ScopeKind)
    -> Option<GraphRefMut<'a, Variable<'a>>>
  {
    if kind.contains(self.kind) {
      if let Some(v) = self.vars.get(name) {
        return Some(v.asleep_mut());
      }
    }
    if kind.contains(ScopeKind::RECURSIVE) {
      self.parent
        .map(|p| p.awake().find_filtered_mut(name, kind))
        .unwrap_or(None)
    } else {
      None
    }
  }

  pub fn find_filtered(&self, name: &str, kind: ScopeKind)
    -> Option<GraphRef<'a, Variable<'a>>>
  {
    self.find_filtered_mut(name, kind).map(|v| v.asleep_ref())
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

  pub fn kind(&self) -> ScopeKind {
    self.kind
  }

  pub fn level(&self) -> u32 {
    self.parent.map(|p| 1 + p.awake().level()).unwrap_or(0)
  }
}

impl<'a> Display for Scope<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{} scope ({})", self.kind, self.span)
  }
}

impl<'a> SourceItem for Scope<'a> {
  fn span(&self) -> &TokenSpan {
    &self.span
  }

  fn resolve(&mut self) -> Result<()> {
    for var in self.vars.values_mut() {
      var.awake_mut().resolve()?;
    }
    Ok(())
  }

  fn typecheck(&mut self) -> Result<()> {
    for var in self.vars.values_mut() {
      var.awake_mut().typecheck()?;
    }
    Ok(())
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
  fn scope(&self) -> GraphRef<'a, Scope<'a>> {
    self.scope_mut().asleep_ref()
  }
  fn scope_mut(&self) -> GraphRefMut<'a, Scope<'a>>;
}

#[derive(Debug, Serialize)]
pub struct ScopeFilter<'a> {
  scope: GraphRef<'a, Scope<'a>>,
  kind: ScopeKind,
}

impl<'a> ScopeFilter<'a> {
  pub fn new(
    scope: GraphRef<'a, Scope<'a>>,
    kind: ScopeKind,
  ) -> Self
  {
    ScopeFilter { scope, kind }
  }

  pub fn to_inner(&self) -> GraphRef<'a, Scope<'a>> {
    self.scope
  }

  pub fn kind(&self) -> ScopeKind {
    self.kind
  }

  pub fn set_kind(&mut self, kind: ScopeKind) {
    self.kind = kind;
  }
}

impl<'a> From<GraphRef<'a, Scope<'a>>> for ScopeFilter<'a> {
  fn from(scope: GraphRef<'a, Scope<'a>>) -> Self {
    let kind = scope.awake().kind();
    ScopeFilter::new(scope, kind)
  }
}

impl<'a> Display for ScopeFilter<'a> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "scope filter {}", self.kind)
  }
}

impl<'a> Owner<'a, Variable<'a>> for ScopeFilter<'a> {
  fn find_mut(&self, name: &str) -> Option<GraphRefMut<'a, Variable<'a>>> {
    self.scope.awake().find_filtered_mut(name, self.kind)
  }
}
