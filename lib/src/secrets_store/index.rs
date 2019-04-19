use crate::api::{SecretEntry, SecretVersion};
use crate::block_store::{Change, ChangeLog, Operation};
use crate::secrets_store::SecretStoreResult;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexEntry {
  entry: SecretEntry,
  blocks: Vec<String>,
  current_block: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Index {
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
          Some(existing) => {
            existing.blocks.push(block_id.to_string());
            if existing.entry.timestamp < version.timestamp {
              existing.entry.name = version.name;
              existing.entry.secret_type = version.secret_type;
              existing.entry.tags = version.tags;
              existing.entry.urls = version.urls;
              existing.entry.timestamp = version.timestamp;
              existing.entry.deleted = version.deleted;
              existing.current_block = block_id.to_string();
            }
          }
          None => {
            let entry = IndexEntry {
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
            };

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
          entry = Self::create_entry(&entry.blocks, &version_accessor)?;
        }
        self.entries.insert(entry.entry.id.clone(), entry);
      }
    }

    Ok(true)
  }

  fn create_entry<F>(block_ids: &[String], version_accessor: F) -> SecretStoreResult<IndexEntry>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    unimplemented!()
  }
}
