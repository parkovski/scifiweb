use std::sync::Arc;
use fxhash::FxHashMap;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use compile::{TokenSpan, TokenValue};
use util::InsertGraphCell;
use util::graph_cell::*;
use super::*;
use super::errors::*;
use super::ty::*;

#[derive(Debug, Serialize)]
pub struct Variable<'a> {
  name: TokenValue<Arc<str>>,
  ty: ItemRef<'a, Type<'a>>,
}

impl<'a> Variable<'a> {
  pub fn new(
    name: TokenValue<Arc<str>>,
    ty: ItemRef<'a, Type<'a>>,
  ) -> Self
  {
    Variable { name, ty }
  }

  pub fn ty(&self) -> Option<GraphRef<Type<'a>>> {
    self.ty.item()
  }
}

impl_named!(Variable, "variable", <'a>);
named_display!(Variable, <'a>);

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

#[derive(Debug)]
pub struct Scope<'a> {
  vars: FxHashMap<Arc<str>, GraphCell<Variable<'a>>>,
  parent: Option<GraphRefMut<'a, Scope<'a>>>,
}

impl<'a> Scope<'a> {
  pub fn new() -> GraphCell<Self> {
    GraphCell::new(Scope {
      vars: Default::default(),
      parent: None,
    })
  }

  pub fn child(this: GraphRefMut<'a, Scope<'a>>) -> GraphCell<Self> {
    GraphCell::new(Scope {
      vars: Default::default(),
      parent: Some(this),
    })
  }

  pub fn end(&self, current: &mut GraphRefMut<'a, Scope<'a>>) -> Result<()> {
    if let Some(parent) = self.parent {
      *current = parent;
      Ok(())
    } else {
      Err(ErrorKind::InvalidOperation("can't end the global scope").into())
    }
  }

  pub fn set_parent(&mut self, parent: GraphRefMut<'a, Scope<'a>>) -> Result<()> {
    if self.parent.is_some() {
      return Err(ErrorKind::InvalidOperation(
        "can't set parent on scope that already has a parent"
      ).into());
    }
    let p = parent.awake();
    for (key, value) in &self.vars {
      if p.has_var(key) {
        return Err(ErrorKind::DuplicateDefinition(
          value.awake().name().clone(), "variable"
        ).into());
      }
    }
    Ok(self.parent = Some(parent))
  }

  pub fn has_var(&self, name: &str) -> bool {
    self.vars.contains_key(name) || self.parent.map_or(false, |p| p.awake().has_var(name))
  }

  pub fn insert_var(&mut self, var: Variable<'a>) -> Result<GraphRefMut<'a, Variable<'a>>> {
    let error: Error = ErrorKind::DuplicateDefinition(
        var.name().clone(), "variable"
    ).into();
    if let Some(parent) = self.parent {
      if parent.awake().has_var(&var.name()) {
        return Err(error);
      }
    }
    self.vars
      .insert_graph_cell(var.name().value().clone(), var)
      .map_err(move |_| error)
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
