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

/// Implement traits for types that are identified
/// by a unique name so they can be compared and
/// be stored and looked up by name in a hash set.
macro_rules! impl_name_traits {
  ($ty:ident, $($bounds:tt)*) => (
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
  (@all $ty:ident, $($bounds:tt)*) => (
    impl_name_traits!($ty, $($bounds)*);

    impl $($bounds)* Eq for generic_name_short!($ty $($bounds)*) {}

    impl $($bounds)* Ord for generic_name_short!($ty $($bounds)*) {
      fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        self.name().cmp(other.name())
      }
    }
  );
}

macro_rules! named_display {
  ($ty:ident, $($bounds:tt)*) => (
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
  ($ty:ident) => (
    named_display!($ty,);
  )
}

macro_rules! impl_named {
  ($ty:ident, $item_name:expr, $($bounds:tt)*) => (
    impl $($bounds)* ::ast::Named for generic_name_short!($ty $($bounds)*) {
      fn name(&self) -> &::compile::TokenValue<::std::sync::Arc<str>> {
        &self.name
      }

      fn item_name(&self) -> &'static str {
        $item_name
      }
    }
  );
  (type $ty:ident, $($bounds:tt)*) => (
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
