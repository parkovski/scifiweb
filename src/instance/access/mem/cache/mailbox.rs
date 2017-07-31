use std::collections::{BTreeMap, HashMap};
use std::collections::hash_map::Entry as HEntry;
use std::collections::btree_map::Entry as BTEntry;

use instance::Target;
use instance::mailbox::{Mailbox, MailboxError, Message, MessageThread};

pub struct MailboxCache {
  // Mailbox ID to mailbox
  mailboxes: BTreeMap<u64, Mailbox>,
  /// Owner -> name -> indexes into mailboxes
  mailbox_owner_map: HashMap<Target, BTreeMap<String, u64>>,
}

impl MailboxCache {
  pub fn new() -> Self {
    MailboxCache {
      mailboxes: BTreeMap::new(),
      mailbox_owner_map: HashMap::new(),
    }
  }

  pub fn put_mailbox(&mut self, mailbox: Mailbox) -> Result<Mailbox, MailboxError> {
    trace!(
      "Storing mailbox #{} ({} for {})",
      mailbox.id(),
      mailbox.name(),
      mailbox.owner()
    );
    let name_map_entry = self
      .mailbox_owner_map
      .entry(mailbox.owner())
      .or_insert_with(BTreeMap::new)
      .entry(mailbox.name().to_owned());
    let id_map_entry = self.mailboxes.entry(mailbox.id());
    match (name_map_entry, id_map_entry) {
      (_, BTEntry::Occupied(_)) => Err(MailboxError::already_exists("Mailbox ID", mailbox.id())),
      (BTEntry::Occupied(_), _) => {
        Err(MailboxError::already_exists("Mailbox name", mailbox.name()))
      }
      (BTEntry::Vacant(ne), BTEntry::Vacant(ie)) => {
        ne.insert(mailbox.id());
        ie.insert(mailbox.clone());
        Ok(mailbox)
      }
    }
  }

  pub fn get_mailbox_for_owner(&self, owner: Target, name: &str) -> Option<Mailbox> {
    self
      .mailbox_owner_map
      .get(&owner)
      .and_then(|name_map| name_map.get(name))
      .and_then(|id| self.mailboxes.get(id))
      .and_then(|mailbox| Some(mailbox.clone()))
  }

  pub fn get_mailbox_by_id(&self, id: u64) -> Option<Mailbox> {
    self.mailboxes.get(&id).cloned()
  }

  pub fn get_mailbox_by_id_mut(&mut self, id: u64) -> Option<&mut Mailbox> {
    self.mailboxes.get_mut(&id)
  }

  pub fn get_all_mailboxes(&self, owner: Target) -> Option<Vec<Mailbox>> {
    self.mailbox_owner_map.get(&owner).and_then(|name_map| {
      let mut values = name_map
        .values()
        .map(|v| self.mailboxes.get(v))
        .filter_map(|option_mailbox| option_mailbox)
        .cloned();
      if values.any(|_| true) {
        Some(values.collect())
      } else {
        None
      }
    })
  }

  /// Returns thread IDs if successful
  pub fn delete_mailbox_for_owner(
    &mut self,
    owner: Target,
    name: &str,
  ) -> Result<Vec<u64>, MailboxError> {
    let mut name_map_entry = match self.mailbox_owner_map.entry(owner.clone()) {
      HEntry::Occupied(e) => e,
      HEntry::Vacant(_) => return Err(MailboxError::not_found("entry for owner", owner)),
    };
    let result = if let BTEntry::Occupied(e) = name_map_entry.get_mut().entry(String::from(name)) {
      trace!("Deleting mailbox {} for {}", name, owner);
      let id = e.remove();
      self
        .mailboxes
        .remove(&id)
        .map(|mb| mb.thread_ids)
        .ok_or_else(|| MailboxError::not_found("mailbox id", id))
    } else {
      return Err(MailboxError::not_found("mailbox name map entry", name));
    };
    // If no more mailboxes are left for this owner, remove the map.
    if name_map_entry.get().is_empty() {
      name_map_entry.remove();
    }
    result
  }

  /// Returns thread IDs if successful
  pub fn delete_mailbox_by_id(&mut self, id: u64) -> Result<Vec<u64>, MailboxError> {
    let mailbox = match self.mailboxes.remove(&id) {
      Some(mb) => mb,
      None => {
        trace!("Delete mailbox #{} - not found", id);
        return Err(MailboxError::not_found("mailbox id", id));
      }
    };
    trace!(
      "Deleted mailbox #{} ({} for {})",
      id,
      mailbox.name(),
      mailbox.owner()
    );
    let mut name_map_entry = match self.mailbox_owner_map.entry(mailbox.owner()) {
      HEntry::Occupied(e) => e,
      HEntry::Vacant(_) => {
        warn!("Mailbox #{} was found but not in a name map", id);
        return Ok(mailbox.thread_ids);
      }
    };
    if let BTEntry::Occupied(e) = name_map_entry.get_mut().entry(mailbox.name().to_owned()) {
      e.remove();
    }
    if name_map_entry.get().is_empty() {
      name_map_entry.remove();
    }
    Ok(mailbox.thread_ids)
  }

  /// Returns thread IDs for all mailboxes found.
  pub fn delete_all_mailboxes(&mut self, owner: Target) -> Result<Vec<u64>, MailboxError> {
    trace!("Deleting all mailboxes for {}", owner);
    let name_map_entry = match self.mailbox_owner_map.entry(owner.clone()) {
      HEntry::Occupied(e) => e,
      HEntry::Vacant(_) => {
        return Err(MailboxError::not_found("name map entry", owner));
      }
    };
    let mailboxes = &mut self.mailboxes;
    let mut ids = Vec::new();
    for id in name_map_entry.get().values() {
      if let Some(mailbox) = mailboxes.remove(id) {
        ids.extend(mailbox.thread_ids);
      }
    }
    name_map_entry.remove();
    Ok(ids)
  }
}

pub struct MessageThreadCache {
  threads: BTreeMap<u64, MessageThread>,
}

impl MessageThreadCache {
  pub fn new() -> Self {
    MessageThreadCache {
      threads: BTreeMap::new(),
    }
  }

  pub fn put_thread(&mut self, thread: MessageThread) -> Result<MessageThread, MailboxError> {
    match self.threads.entry(thread.id) {
      BTEntry::Occupied(_) => Err(MailboxError::already_exists("thread id", thread.id)),
      BTEntry::Vacant(e) => {
        e.insert(thread.clone());
        Ok(thread)
      }
    }
  }

  pub fn get_thread_by_id(&self, id: u64) -> Option<MessageThread> {
    self.threads.get(&id).cloned()
  }

  pub fn get_thread_by_id_mut(&mut self, id: u64) -> Option<&mut MessageThread> {
    self.threads.get_mut(&id)
  }

  /// The returned Vec is in the same order as ids, with None
  /// in place of the ones not found.
  pub fn get_threads_by_id(&self, ids: &[u64]) -> Vec<Option<MessageThread>> {
    ids.iter().map(|id| self.threads.get(id).cloned()).collect()
  }

  /// Returns the message IDs of all the deleted threads.
  /// Ignores missing thread IDs.
  pub fn delete_threads(&mut self, ids: &[u64]) -> Vec<u64> {
    ids.into_iter().fold(Vec::new(), |mut ids, id| {
      match self.threads.remove(id) {
        Some(thread) => {
          ids.extend(&thread.message_ids);
        }
        None => {
          debug!("Thread id {} missing", id);
        }
      }
      ids
    })
  }
}

pub struct MessageCache {
  messages: BTreeMap<u64, Message>,
}

impl MessageCache {
  pub fn new() -> Self {
    MessageCache {
      messages: BTreeMap::new(),
    }
  }

  pub fn put_message(&mut self, message: Message) -> Result<Message, MailboxError> {
    match self.messages.entry(message.id) {
      BTEntry::Occupied(_) => Err(MailboxError::already_exists("message id", message.id)),
      BTEntry::Vacant(e) => {
        e.insert(message.clone());
        Ok(message)
      }
    }
  }

  pub fn get_messages_by_id(&self, ids: &[u64]) -> Vec<Option<Message>> {
    ids
      .iter()
      .map(|id| self.messages.get(id).cloned())
      .collect()
  }

  /// Ignores missing messages.
  pub fn delete_messages(&mut self, ids: &[u64]) {
    for id in ids {
      self.messages.remove(id);
    }
  }
}
