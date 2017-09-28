/// Removes the bounds from a generic name, so other macros can make
/// impl blocks without repeating them.
macro_rules! generic_name_short {
  ($first:ident $($rest:tt)+) => (
    generic_name_short!(@name ($($rest)+), ($first))
  );
  (:: $($rest:tt)+) => (
    generic_name_short!(@name ($($rest)+), (::))
  );
  (@name (< $($rest:tt)+), ($($name:tt)+)) => (
    generic_name_short!(@append ($($rest)+), ($($name)+), ())
  );
  (@name ($t:tt $($rest:tt)*), ($($name:tt)+)) => (
    generic_name_short!(@name ($($rest)*), ($($name)+ $t))
  );
  (@name (), ($($name:tt)+)) => (
    $($name)+
  );
  (@append (>), ($($name:tt)+), ($($bounds:tt)+)) => (
    $($name)+ < $($bounds)+ >
  );
  (@append (: $($rest:tt)+), ($($name:tt)+), ($($bounds:tt)+)) => (
    generic_name_short!(@skip ($($rest)+), ($($name)+), ($($bounds)+))
  );
  (@append ($first:tt $($rest:tt)+), ($($name:tt)+), ($($bounds:tt)*)) => (
    generic_name_short!(@append ($($rest)+), ($($name)+), ($($bounds)* $first))
  );
  (@skip (, $($rest:tt)+), ($($name:tt)+), ($($bounds:tt)+)) => (
    generic_name_short!(@append ($($rest)+), ($($name)+), ($($bounds)+,))
  );
  (@skip (> $($rest:tt)*), ($($name:tt)+), ($($bounds:tt)+)) => (
    generic_name_short!(@append (> $($rest)*), ($($name)+), ($($bounds)+))
  );
  (@skip ($first:tt $($rest:tt)+), ($($name:tt)+), ($($bounds:tt)+)) => (
    generic_name_short!(@skip ($($rest)+), ($($name)+), ($($bounds)+))
  );
}

macro_rules! type_macros {
  (@make $ty:ident($($bounds:tt)+), $m:ident ($($args:tt)*)) => (
    $m!($($args)* $ty<$($bounds)+>);
  );
  (@make $ty:ident(), $m:ident ($($args:tt)*)) => (
    $m!($($args)* $ty);
  );
  (@gen $ty:ident ($($bounds:tt)+) >; $($rest:tt)+) => (
    type_macros!(@process $ty ($($bounds)+), $($rest)+);
  );
  (@gen ty:ident ($($bounds:tt)+) $next:tt $($rest:tt)+) => (
    type_macros!(@gen $ty ($($bounds)+ $next) $($rest)+);
  );
  (@process $ty:ident $bounds:tt, $first:ident ($($args:tt)*) $($rest:tt)*) => (
    type_macros!(@make $ty $bounds, $first($($args)*));
    type_macros!(@process $ty $bounds $($rest)*);
  );
  (@process $ty:ident $bounds:tt, $m:ident) => (
    type_macros!(@process $ty $bounds, $m());
  );
  (@process $ty:ident $bounds:tt, $first:ident, $($rest:tt)+) => (
    type_macros!(@process $ty $bounds, $first(), $($rest)+);
  );
  (@process $ty:ident $b:tt) => ();
  ($ty:ident; $($macros:tt)+) => (
    type_macros!(@process $ty (), $($macros)+);
  );
  ($ty:ident< $first:tt $($rest:tt)+) => (
    type_macros!(@gen $ty ($first) $($rest)+);
  );
  ($ty:ident ($($bounds:tt)+); $($rest:tt)+) => (
    type_macros!(@process $ty ($($bounds)+), $($rest)+);
  );
}

/// Implement traits for types that are identified
/// by a unique name so they can be compared and
/// be stored and looked up by name in a hash set.
macro_rules! impl_name_traits {
  ($ty:ident $($bounds:tt)*) => (
    impl $($bounds)* PartialEq for generic_name_short!($ty $($bounds)*) {
      fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
      }
    }

    impl $($bounds)* PartialOrd for generic_name_short!($ty $($bounds)*) {
      fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        self.name().partial_cmp(other.name())
      }
    }

    impl $($bounds)* ::std::hash::Hash for generic_name_short!($ty $($bounds)*) {
      fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state)
      }
    }

    impl $($bounds)* ::std::borrow::Borrow<str> for generic_name_short!($ty $($bounds)*) {
      fn borrow(&self) -> &str {
        self.name()
      }
    }
  );
  (@all $ty:ident $($bounds:tt)*) => (
    impl_name_traits!($ty $($bounds)*);

    impl $($bounds)* Eq for generic_name_short!($ty $($bounds)*) {}

    impl $($bounds)* Ord for generic_name_short!($ty $($bounds)*) {
      fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        self.name().cmp(other.name())
      }
    }
  );
}

macro_rules! named_display {
  ($ty:ident $($bounds:tt)*) => (
    impl $($bounds)* ::std::fmt::Display for generic_name_short!($ty $($bounds)*) {
      fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        let item_name = self.item_name();
        if item_name.is_empty() {
          write!(f, "{}", self.name().value())
        } else {
          write!(f, "{} {}", item_name, self.name().value())
        }
      }
    }
  );
}

macro_rules! impl_named {
  ($item_name:expr, $ty:ident $($bounds:tt)*) => (
    impl $($bounds)* ::ast::Named for generic_name_short!($ty $($bounds)*) {
      fn name(&self) -> &::compile::TokenValue<::std::sync::Arc<str>> {
        &self.name
      }

      fn item_name(&self) -> &'static str {
        $item_name
      }
    }
  );
  (type $ty:ident $($bounds:tt)*) => (
    impl $($bounds)* ::ast::Named for generic_name_short!($ty $($bounds)*) {
      fn name(&self) -> &::compile::TokenValue<::std::sync::Arc<str>> {
        &self.name
      }

      fn item_name(&self) -> &'static str {
        Self::BASE_TYPE.as_str()
      }
    }
  );
}

macro_rules! impl_scoped {
  ($lifetime:tt, $ty:ident $($bounds:tt)*) => (
    impl $($bounds)* ::ast::var::Scoped<$lifetime> for generic_name_short!($ty $($bounds)*) {
      fn scope(&self)
        -> ::util::graph_cell::GraphRef<$lifetime, ::ast::var::Scope<$lifetime>>
      {
        self.scope.asleep()
      }

      fn scope_mut(&self)
        -> ::util::graph_cell::GraphRefMut<$lifetime, ::ast::var::Scope<$lifetime>>
      {
        self.scope.asleep_mut()
      }
    }
  )
}
