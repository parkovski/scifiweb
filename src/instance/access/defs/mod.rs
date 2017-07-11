pub mod mailbox;
pub use self::mailbox::*;

pub trait Accessor<'a>
  : MailboxAccessor<'a> + MessageThreadAccessor<'a> + MessageAccessor<'a>
{}

impl<'a, T> Accessor<'a> for T where T
  : MailboxAccessor<'a> + MessageThreadAccessor<'a> + MessageAccessor<'a>
{}