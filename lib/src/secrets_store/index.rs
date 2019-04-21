use crate::api::{SecretEntry, SecretEntryMatch, SecretListFilter, SecretVersion};
use crate::block_store::{Change, ChangeLog, Operation};
use crate::memguard::weak::ZeroingHHeapAllocator;
use crate::memguard::SecretWords;
use crate::secrets_store::SecretStoreResult;
use crate::secrets_store_capnp::index;
use capnp::{message, serialize};
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexEntry {
  entry: SecretEntry,
  blocks: Vec<String>,
  current_block: String,
}

impl IndexEntry {
  fn new(block_id: &str, version: SecretVersion) -> IndexEntry {
    IndexEntry {
      entry: SecretEntry {
        id: version.secret_id,
        name: version.name,
        secret_type: version.secret_type,
        tags: version.tags,
        urls: version.urls,
        timestamp: version.timestamp,
        deleted: version.deleted,
      },
      blocks: vec![block_id.to_string()],
      current_block: block_id.to_string(),
    }
  }

  fn add_version(&mut self, block_id: &str, version: SecretVersion) {
    self.blocks.push(block_id.to_string());
    if self.entry.timestamp < version.timestamp {
      self.entry.name = version.name;
      self.entry.secret_type = version.secret_type;
      self.entry.tags = version.tags;
      self.entry.urls = version.urls;
      self.entry.timestamp = version.timestamp;
      self.entry.deleted = version.deleted;
      self.current_block = block_id.to_string();
    }
  }
}

pub struct Index {
  data: SecretWords,
  heads: HashMap<String, Change>,
  entries: HashMap<String, IndexEntry>,
}

impl Index {
  fn from_raw(raw: &mut [u8]) -> SecretStoreResult<Index> {
    let data = SecretWords::from(raw);
    let heads = Self::current_heads(&data)?;

    Ok(Index {
      data,
      heads,
      entries: HashMap::new(),
    })
  }

  pub fn process_change_logs<F>(&mut self, change_logs: &[ChangeLog], version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let mut changed = false;

    for change_log in change_logs {
      if self.process_change_log(change_log, &version_accessor)? {
        changed = true;
      }
    }
    Ok(changed)
  }

  pub fn filter_entries(filter: &SecretListFilter) -> Vec<SecretEntryMatch> {
    unimplemented!()
  }

  pub fn process_change_logs2<F>(&mut self, change_logs: &[ChangeLog], version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let (new_heads, added_versions, deleted_blocks) = self.collect_changes(change_logs, version_accessor)?;

    if added_versions.is_empty() && deleted_blocks.is_empty() {
      // No change that affects us
      return Ok(false);
    }

    let data_borrow = self.data.borrow();
    let reader = serialize::read_message_from_words(&data_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut to_keep = HashSet::new();
    let mut to_remove = HashSet::new();

    for entry in index.get_entries()? {
      let secret_id = entry.get_id()?;
      let mut remainging_count = 0;
      for maybe_block_id in entry.get_block_ids()? {
        let block_id = maybe_block_id?;
        if !deleted_blocks.contains(block_id) {
          remainging_count += 1
        }
      }
      if remainging_count > 0 {
        to_keep.insert(secret_id.to_string());
      } else {
        to_remove.insert(secret_id.to_string());
      }
    }

    unimplemented!()
  }

  fn process_change_log<F>(&mut self, change_log: &ChangeLog, version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    self.process_changes(
      change_log.changes_since(self.heads.get(&change_log.node)),
      version_accessor,
    )
  }

  fn process_changes<'a, F, I>(&mut self, changes: I, version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
    I: Iterator<Item = &'a Change>,
  {
    let mut changed = false;

    for change in changes {
      let relevant_change = match change.op {
        Operation::Add => self.process_block_added(&change.block, &version_accessor)?,
        Operation::Delete => self.process_block_deleted(&change.block, &version_accessor)?,
      };
      if relevant_change {
        changed = true;
      }
    }

    Ok(changed)
  }

  fn process_block_added<'a, F>(&mut self, block_id: &str, version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    match version_accessor(block_id)? {
      Some(version) => {
        match self.entries.get_mut(&version.secret_id) {
          Some(existing) => existing.add_version(block_id, version),
          None => {
            let entry = IndexEntry::new(block_id, version);

            self.entries.insert(entry.entry.id.clone(), entry);
          }
        }
        Ok(true)
      }
      _ => Ok(false), // That version was not for us
    }
  }

  fn process_block_deleted<'a, F>(&mut self, block_id: &String, version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let dirty_entry_ids: Vec<String> = self
      .entries
      .values()
      .filter(|e| e.blocks.contains(block_id))
      .map(|e| e.entry.id.clone())
      .collect();

    if dirty_entry_ids.is_empty() {
      return Ok(false);
    }

    for dirty_entry_id in dirty_entry_ids {
      if let Some(mut entry) = self.entries.remove(&dirty_entry_id) {
        entry.blocks = entry.blocks.iter().filter(|b| *b != block_id).cloned().collect();
        if entry.blocks.is_empty() {
          continue; // This was the only block in the entry, so we can keep it removed
        }
        if &entry.current_block == block_id {
          // Need to recreate the entry to figure out the new current block
          if let Some(new_entry) = Self::create_entry(&entry.blocks, &version_accessor)? {
            self.entries.insert(new_entry.entry.id.clone(), new_entry);
          }
        } else {
          self.entries.insert(entry.entry.id.clone(), entry);
        }
      }
    }

    Ok(true)
  }

  fn create_entry<F>(block_ids: &[String], version_accessor: F) -> SecretStoreResult<Option<IndexEntry>>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let mut versions = Vec::with_capacity(block_ids.len());

    for block_id in block_ids {
      if let Some(version) = version_accessor(block_id)? {
        versions.push((block_id, version))
      }
    }

    let mut entry = match versions.pop() {
      Some((block_id, version)) => IndexEntry::new(block_id, version),
      None => return Ok(None),
    };
    for (block_id, version) in versions {
      entry.add_version(block_id, version)
    }

    Ok(Some(entry))
  }

  fn collect_changes<F>(
    &mut self,
    change_logs: &[ChangeLog],
    version_accessor: F,
  ) -> SecretStoreResult<(
    HashMap<String, Change>,
    HashMap<String, HashMap<String, SecretVersion>>,
    HashSet<String>,
  )>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let mut new_heads = HashMap::with_capacity(change_logs.len());
    let mut added_versions = HashMap::new();
    let mut deleted_blocks = HashSet::new();

    for change_log in change_logs {
      let changes = change_log.changes_since(self.heads.get(&change_log.node));

      for change in changes {
        match change.op {
          Operation::Add => {
            if let Some(secret_version) = version_accessor(&change.block)? {
              let secret_id = secret_version.secret_id.clone();
              let mut by_blocks = added_versions.remove(&secret_id).unwrap_or_else(|| HashMap::new());
              by_blocks.insert(change.block.clone(), secret_version);
              added_versions.insert(secret_id, by_blocks);
            }
          }
          Operation::Delete => {
            deleted_blocks.insert(change.block.clone());
          }
          _ => (),
        }
      }

      if let Some(last) = change_log.changes.last() {
        new_heads.insert(change_log.node.clone(), last.clone());
      }
    }

    // Note there are usually more added blocks then deleted, so it is reasonable to do it this way
    for deleted_block in &deleted_blocks {
      for by_block in added_versions.values_mut() {
        by_block.remove(deleted_block);
      }
    }

    Ok((new_heads, added_versions, deleted_blocks))
  }

  fn current_heads(index_data: &SecretWords) -> SecretStoreResult<HashMap<String, Change>> {
    let index_borrow = index_data.borrow();
    let reader = serialize::read_message_from_words(&index_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut heads = HashMap::with_capacity(index.get_heads()?.len() as usize);

    for head in index.get_heads()? {
      let node_id = head.get_node_id()?.to_string();
      let op = match head.get_operation()? {
        index::HeadOperation::Add => Operation::Add,
        index::HeadOperation::Delete => Operation::Delete,
      };
      let block = head.get_block_id()?.to_string();
      heads.insert(node_id, Change { op, block });
    }

    Ok(heads)
  }
}

impl Default for Index {
  fn default() -> Self {
    let mut index_message = message::Builder::new(ZeroingHHeapAllocator::new());
    index_message.init_root::<index::Builder>();
    let mut index_data = serialize::write_message_to_words(&index_message);

    Index {
      data: index_data.into(),
      heads: HashMap::new(),
      entries: HashMap::new(),
    }
  }
}
