pub mod messaging;

use self::messaging::MessagingAccessor;

pub trait Accessor<'a>: Clone + MessagingAccessor<'a> {}

impl<'a, A> Accessor<'a> for A
where
  A: Clone + MessagingAccessor<'a>,
{
}
