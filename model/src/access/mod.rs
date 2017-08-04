pub mod messaging;

use self::messaging::MessagingAccessor;

pub trait Accessor<'a>: MessagingAccessor<'a> {}

impl<'a, A> Accessor<'a> for A
where
  A: MessagingAccessor<'a>,
{}

/// Weird object safety stuff
pub trait ClonableAccessor<'a>: Accessor<'a> + Clone {}

impl<'a, A> ClonableAccessor<'a> for A
where
  A: Accessor<'a> + Clone,
{}
