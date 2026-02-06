use std::{collections::HashMap, path::Path};

use sled::transaction::{ConflictableTransactionError, Transactional};

use crate::{block_store::Operation, memguard::weak::ZeroingWords};

use super::{BlockStore, Change, ChangeLog, RingContent, RingId, StoreError, StoreResult};

#[derive(Debug)]
pub struct SledBlockStore {
  node_id: String,
  db: sled::Db,
  rings: sled::Tree,
  indices: sled::Tree,
  blocks: sled::Tree,
  change_logs: sled::Tree,
}

impl SledBlockStore {
  pub fn new<P: AsRef<Path>>(db_file: P, node_id: &str) -> StoreResult<SledBlockStore> {
    let db = sled::open(db_file)?;
    let rings = db.open_tree("rings")?;
    let indices = db.open_tree("indices")?;
    let blocks = db.open_tree("blocks")?;
    let change_logs = db.open_tree("change_logs")?;

    Ok(SledBlockStore {
      node_id: node_id.to_string(),
      db,
      rings,
      indices,
      blocks,
      change_logs,
    })
  }

  fn list_ring_versions(&self) -> StoreResult<HashMap<String, (u64, String)>> {
    let mut ring_versions: HashMap<String, (u64, String)> = HashMap::new();

    for key in self.rings.iter().keys() {
      let key = String::from_utf8_lossy(key?.as_ref()).to_string();
      let mut parts = key.split('.');
      let name = parts.next().map(str::to_string).unwrap_or_else(|| key.clone());
      let version = parts
        .next()
        .and_then(|version_str| version_str.parse::<u64>().ok())
        .unwrap_or_default();

      if let Some((current, _)) = ring_versions.get(&name) {
        if *current > version {
          continue;
        }
      }
      ring_versions.insert(name, (version, key));
    }
    Ok(ring_versions)
  }
}

impl Drop for SledBlockStore {
  fn drop(&mut self) {
    // Note: Not really necessary, just to be on the save side (pun intended)
    self.db.flush().ok();
  }
}

impl BlockStore for SledBlockStore {
  fn node_id(&self) -> &str {
    &self.node_id
  }

  fn list_ring_ids(&self) -> StoreResult<Vec<RingId>> {
    Ok(
      self
        .list_ring_versions()?
        .into_iter()
        .map(|(id, (version, _))| (id, version))
        .collect(),
    )
  }

  fn get_ring(&self, ring_id: &str) -> StoreResult<RingContent> {
    match self.list_ring_versions()?.get(ring_id) {
      Some((version, key)) => self
        .rings
        .get(key)?
        .map(|ring| (*version, ring.as_ref().into()))
        .ok_or_else(|| StoreError::InvalidBlock(ring_id.to_string())),
      None => Err(StoreError::InvalidBlock(ring_id.to_string())),
    }
  }

  fn store_ring(&self, ring_id: &str, version: u64, raw: &[u8]) -> StoreResult<()> {
    if self
      .rings
      .compare_and_swap::<String, &[u8], &[u8]>(format!("{}.{}", ring_id, version), None, Some(raw))?
      .is_err()
    {
      return Err(StoreError::Conflict(format!(
        "Ring {} with version {} already exists",
        ring_id, version
      )));
    }
    self.rings.flush()?;
    Ok(())
  }

  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>> {
    self
      .change_logs
      .iter()
      .map(|kv| {
        let (k, v) = kv?;

        let changes: Vec<Change> = rmp_serde::from_read(v.as_ref())?;
        Ok(ChangeLog {
          node: String::from_utf8_lossy(&k).to_string(),
          changes,
        })
      })
      .collect()
  }

  fn get_index(&self, index_id: &str) -> StoreResult<Option<ZeroingWords>> {
    Ok(self.indices.get(index_id)?.map(|index| index.as_ref().into()))
  }

  fn store_index(&self, index_id: &str, raw: &[u8]) -> StoreResult<()> {
    self.indices.insert(index_id, raw)?;
    self.indices.flush()?;
    Ok(())
  }

  fn insert_block(&self, block_id: &str, node_id: &str, raw: &[u8]) -> StoreResult<()> {
    (&self.blocks, &self.change_logs).transaction(|(blocks, change_logs)| {
      let existing_block = blocks.insert(block_id, raw)?;
      if existing_block.is_some() {
        return Err(ConflictableTransactionError::Abort(StoreError::Conflict(
          block_id.to_string(),
        )));
      }
      let change = Change {
        op: Operation::Add,
        block: block_id.to_string(),
      };
      let new_changes = match change_logs.get(node_id)? {
        Some(existing_raw) => {
          let mut existing: Vec<Change> = rmp_serde::from_read(existing_raw.as_ref())
            .map_err(|e| ConflictableTransactionError::Abort(StoreError::from(e)))?;
          if existing.contains(&change) {
            return Err(ConflictableTransactionError::Abort(StoreError::Conflict(
              "Change already committed".to_string(),
            )));
          }
          existing.push(change);
          existing
        }
        None => vec![change],
      };
      let raw =
        rmp_serde::to_vec_named(&new_changes).map_err(|e| ConflictableTransactionError::Abort(StoreError::from(e)))?;
      change_logs.insert(self.node_id.as_str(), raw)?;

      blocks.flush();
      change_logs.flush();

      Ok(())
    })?;
    Ok(())
  }

  fn get_block(&self, block_id: &str) -> StoreResult<ZeroingWords> {
    self
      .blocks
      .get(block_id)?
      .map(|ring| ring.as_ref().into())
      .ok_or_else(|| StoreError::InvalidBlock(block_id.to_string()))
  }

  fn check_block(&self, block_id: &str) -> StoreResult<bool> {
    Ok(self.blocks.contains_key(block_id)?)
  }
}
