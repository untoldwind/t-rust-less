use crate::api::{SecretEntry, SecretEntryMatch, SecretListFilter, SecretVersion};
use crate::block_store::{Change, ChangeLog, Operation};
use crate::memguard::SecretWords;
use crate::secrets_store::SecretStoreResult;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

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

  fn process_change_log<F>(&mut self, change_log: &ChangeLog, version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    match self.heads.get(&change_log.node) {
      Some(head) => self.process_changes(change_log.changes_since(head), version_accessor),
      None => self.process_changes(change_log.changes.iter(), version_accessor),
    }
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
}

impl Default for Index {
  fn default() -> Self {
    Index {
      data: SecretWords::with_capacity(10),
      heads: HashMap::new(),
      entries: HashMap::new(),
    }
  }
}

impl From<&mut [u8]> for Index {
  fn from(bytes: &mut [u8]) -> Self {
    Index {
      data: SecretWords::from(bytes),
      heads: HashMap::new(),
      entries: HashMap::new(),
    }
  }
}
