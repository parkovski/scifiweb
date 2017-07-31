pub mod mailbox;
pub use self::mailbox::*;

pub trait Accessor<'a>: MessagingAccessor<'a> {}

impl<'a, A> Accessor<'a> for A
where
  A: MessagingAccessor<'a>,
{
}
