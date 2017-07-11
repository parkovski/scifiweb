use std::collections::{HashMap, BTreeMap};
use std::collections::hash_map::Entry as HEntry;
use std::collections::btree_map::Entry as BTEntry;

use instance::Target;
use instance::mailbox::{Mailbox, MessageThread, Message};

pub struct MailboxCache<'a> {
  // Mailbox ID to mailbox
  mailboxes: BTreeMap<u64, Mailbox<'a>>,
  /// Owner -> name -> indexes into mailboxes
  mailbox_owner_map: HashMap<Target<'a>, BTreeMap<String, u64>>,
}

impl<'a> MailboxCache<'a> {
  pub fn new() -> Self {
    MailboxCache {
      mailboxes: BTreeMap::new(),
      mailbox_owner_map: HashMap::new(),
    }
  }

  pub fn put_mailbox(&mut self, mailbox: Mailbox<'a>) {
    let name_map = self.mailbox_owner_map.entry(mailbox.owner()).or_insert_with(BTreeMap::new);
    name_map.insert(mailbox.name().clone(), mailbox.id());
    self.mailboxes.insert(mailbox.id(), mailbox);
  }

  pub fn get_mailbox_for_owner(&self, owner: Target<'a>, name: &str) -> Option<Mailbox<'a>> {
    self.mailbox_owner_map.get(&owner)
      .and_then(|name_map| name_map.get(&String::from(name)))
      .and_then(|id| self.mailboxes.get(id))
      .and_then(|mailbox| Some(mailbox.clone()))
  }

  pub fn get_mailbox_by_id(&self, id: u64) -> Option<Mailbox<'a>> {
    self.mailboxes.get(&id).cloned()
  }

  pub fn get_all_mailboxes(&self, owner: Target<'a>) -> Option<Vec<Mailbox<'a>>> {
    self.mailbox_owner_map
      .get(&owner)
      .and_then(|name_map| {
        let mut values = name_map.values()
          .map(|v| self.mailboxes.get(v))
          .filter_map(|option_mailbox| option_mailbox)
          .map(|mailbox_ref| mailbox_ref.clone());
        if values.any(|_| true) {
          Some(values.collect())
        } else {
          None
        }
      })
  }

  /// Returns true if the mailbox was found.
  pub fn delete_mailbox_for_owner(
    &mut self,
    owner: Target<'a>,
    name: &str,
    thread_cache: &mut MessageThreadCache<'a>,
    message_cache: &mut MessageCache,
  ) -> bool
  {
    let mut mailbox_exists = false;
    let mut name_map_entry = match self.mailbox_owner_map.entry(owner) {
      HEntry::Occupied(e) => e,
      HEntry::Vacant(_) => return false,
    };
    if let BTEntry::Occupied(e) = name_map_entry.get_mut().entry(String::from(name)) {
      self.mailboxes
        .remove(&e.remove())
        .map(|mb| {
          mailbox_exists = true;
          Self::clean_up_threads(&mb.thread_ids(), thread_cache, message_cache)
        });
    }
    // If no more mailboxes are left for this owner, remove the map.
    if name_map_entry.get().is_empty() {
      name_map_entry.remove();
    }
    mailbox_exists
  }

  /// Returns true if the mailbox was found.
  pub fn delete_mailbox_by_id(
    &mut self,
    id: u64,
    thread_cache: &mut MessageThreadCache<'a>,
    message_cache: &mut MessageCache,
  ) -> bool
  {
    let mailbox = match self.mailboxes.remove(&id) {
      Some(mb) => mb,
      None => return false,
    };
    Self::clean_up_threads(&*mailbox.thread_ids(), thread_cache, message_cache);
    let mut name_map_entry = match self.mailbox_owner_map.entry(mailbox.owner()) {
      HEntry::Occupied(e) => e,
      HEntry::Vacant(_) => return true,
    };
    if let BTEntry::Occupied(e) = name_map_entry.get_mut().entry(mailbox.name().clone()) {
      e.remove();
    }
    if name_map_entry.get().is_empty() {
      name_map_entry.remove();
    }
    true
  }

  /// Returns true if a name map entry was found for this owner.
  pub fn delete_all_mailboxes(
    &mut self,
    owner: Target<'a>,
    thread_cache: &mut MessageThreadCache<'a>,
    message_cache: &mut MessageCache,
  ) -> bool {
    let name_map_entry = match self.mailbox_owner_map.entry(owner) {
      HEntry::Occupied(e) => e,
      HEntry::Vacant(_) => return false,
    };
    let mailboxes = &mut self.mailboxes;
    Self::clean_up_threads(
      name_map_entry.get().values()
        .filter_map(|id| mailboxes.remove(id))
        .fold(Vec::new(), |mut acc, mailbox| {
          acc.extend(&*mailbox.thread_ids());
          acc
        })
        .as_slice(),
      thread_cache,
      message_cache,
    );
    name_map_entry.remove();
    true
  }

  fn clean_up_threads(
    thread_ids: &[u64],
    thread_cache: &mut MessageThreadCache<'a>,
    message_cache: &mut MessageCache,
  )
  {
    thread_cache.delete_threads(thread_ids, message_cache);
  }
}

pub struct MessageThreadCache<'a> {
  threads: BTreeMap<u64, MessageThread<'a>>,
}

impl<'a> MessageThreadCache<'a> {
  pub fn new() -> Self {
    MessageThreadCache { threads: BTreeMap::new() }
  }

  pub fn put_thread(&mut self, thread: MessageThread<'a>) {
    self.threads.insert(thread.id(), thread);
  }

  pub fn get_thread_by_id(&self, id: u64) -> Option<MessageThread<'a>> {
    self.threads.get(&id).cloned()
  }

  /// The returned Vec is in the same order as ids, with None
  /// in place of the ones not found.
  pub fn get_threads_by_id(&self, ids: &[u64]) -> Vec<Option<MessageThread<'a>>> {
    ids.iter().map(|id| self.threads.get(id).cloned()).collect()
  }

  /// Returns the ids that were not found, or None if all were found.
  pub fn delete_threads(&mut self, ids: &[u64], message_cache: &mut MessageCache) -> Option<Vec<u64>> {
    let (message_ids, missing_thread_ids) = ids
      .into_iter()
      .fold((Vec::new(), Vec::new()), |mut pair, id| {
        {
          let (ref mut message_ids, ref mut missing_thread_ids) = pair;
          match self.threads.remove(id) {
            Some(thread) => { message_ids.extend(&*thread.message_ids()); }
            None => { missing_thread_ids.push(*id); }
          }
        }
        pair
      });

    Self::clean_up_messages(message_ids.as_slice(), message_cache);
    if missing_thread_ids.len() > 0 {
      Some(missing_thread_ids)
    } else {
      None
    }
  }

  fn clean_up_messages(message_ids: &[u64], message_cache: &mut MessageCache) {
    message_cache.delete_messages(message_ids);
  }
}

pub struct MessageCache<'a> {
  messages: BTreeMap<u64, Message<'a>>,
}

impl<'a> MessageCache<'a> {
  pub fn new() -> Self {
    MessageCache {
      messages: BTreeMap::new(),
    }
  }

  pub fn put_message(&mut self, message: Message<'a>) {
    self.messages.insert(message.id(), message);
  }

  pub fn get_messages_by_id(&self, ids: &[u64]) -> Vec<Option<Message<'a>>> {
    ids.iter().map(|id| self.messages.get(id).cloned()).collect()
  }

  /// Returns the ids that were not found, or None if all were found.
  pub fn delete_messages(&mut self, ids: &[u64]) -> Option<Vec<u64>> {
    let mut missing_ids = ids.iter()
      .map(|id| match self.messages.remove(id) {
        None => Some(*id),
        _ => None,
      });
    if missing_ids.all(|id| id.is_none()) {
      None
    } else {
      Some(missing_ids.filter_map(|id| id).collect())
    }
  }
}