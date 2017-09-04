/// Implement traits for types that are identified
/// by a unique name so they can be compared and
/// be stored and looked up by name in a hash set.
macro_rules! impl_name_traits {
  (@std $ty:ident, ($($generic_req:tt)*), ($($generic:tt)*)) => (
    impl $($generic_req)* PartialEq for $ty $($generic)* {
      fn eq(&self, other: &Self) -> bool {
        self.name() == other.name()
      }
    }

    impl $($generic_req)* PartialOrd for $ty $($generic)* {
      fn partial_cmp(&self, other: &Self) -> Option<::std::cmp::Ordering> {
        self.name().partial_cmp(other.name())
      }
    }

    impl $($generic_req)* ::std::hash::Hash for $ty $($generic)* {
      fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self.name().hash(state)
      }
    }

    impl $($generic_req)* ::std::borrow::Borrow<str> for $ty $($generic)* {
      fn borrow(&self) -> &str {
        self.name()
      }
    }
  );
  (@all $ty:ident, ($($generic_req:tt)*), ($($generic:tt)*)) => (
    impl_name_traits!(@std $ty, ($($generic_req)*), ($($generic)*));

    impl $($generic_req)* Eq for $ty $($generic)* {}

    impl $($generic_req)* Ord for $ty $($generic)* {
      fn cmp(&self, other: &Self) -> ::std::cmp::Ordering {
        self.name().cmp(other.name())
      }
    }
  );
  ($ty:ident) => (
    impl_name_traits!(@std $ty, (), ());
  );
  (($($gen_req:tt)+)$ty:ident($($gen:tt)+)) => (
    impl_name_traits!(@std $ty, ($($gen_req)+), ($($gen)+));
  );
  ($ty:ident, all) => (
    impl_name_traits!(@all $ty, (), ());
  );
  (($($gen_req:tt)+)$ty:ident($($gen:tt)+), all) => (
    impl_name_traits!(@all $ty, ($($gen_req)+), ($($gen)+));
  );
}

macro_rules! named_display {
  (($($gen_req:tt)*) $ty:ident ($($gen:tt)*)) => (
    impl $($gen_req)* ::std::fmt::Display for $ty $($gen)* {
      fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        let item_name = self.item_name();
        if item_name.is_empty() {
          write!(f, "{}", self.name())
        } else {
          write!(f, "{} {}", item_name, self.name())
        }
      }
    }
  );
  ($ty:ident) => (
    named_display!(() $ty ());
  )
}
