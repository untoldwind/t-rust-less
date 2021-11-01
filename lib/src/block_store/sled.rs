use std::path::Path;

use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use sled::transaction::ConflictableTransactionError;

use crate::memguard::weak::ZeroingWords;

use super::{BlockStore, Change, ChangeLog, StoreError, StoreResult};

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

  fn generate_id(data: &[u8]) -> String {
    let mut hasher = Sha256::new();

    hasher.update(data);

    HEXLOWER.encode(&hasher.finalize())
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

  fn list_ring_ids(&self) -> StoreResult<Vec<String>> {
    self
      .rings
      .iter()
      .keys()
      .map(|k| Ok(String::from_utf8_lossy(k?.as_ref()).to_string()))
      .collect()
  }

  fn get_ring(&self, ring_id: &str) -> StoreResult<ZeroingWords> {
    self
      .rings
      .get(ring_id)?
      .map(|ring| ring.as_ref().into())
      .ok_or_else(|| StoreError::InvalidBlock(ring_id.to_string()))
  }

  fn store_ring(&self, ring_id: &str, raw: &[u8]) -> StoreResult<()> {
    self.rings.insert(ring_id, raw)?;
    self.rings.flush()?;
    Ok(())
  }

  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>> {
    self
      .change_logs
      .iter()
      .map(|kv| {
        let (k, v) = kv?;

        let changes: Vec<Change> = rmp_serde::decode::from_read(v.as_ref())?;
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

  fn add_block(&self, raw: &[u8]) -> StoreResult<String> {
    let block_id = Self::generate_id(raw);
    self.blocks.insert(&block_id, raw)?;
    self.blocks.flush()?;
    Ok(block_id)
  }

  fn get_block(&self, block: &str) -> StoreResult<ZeroingWords> {
    self
      .blocks
      .get(block)?
      .map(|ring| ring.as_ref().into())
      .ok_or_else(|| StoreError::InvalidBlock(block.to_string()))
  }

  fn commit(&self, changes: &[Change]) -> StoreResult<()> {
    self.change_logs.transaction::<_, _, StoreError>(|tx| {
      let new_changes = match tx.get(&self.node_id)? {
        Some(existing_raw) => {
          let mut existing: Vec<Change> = rmp_serde::from_read(existing_raw.as_ref())
            .map_err(|e| ConflictableTransactionError::Abort(StoreError::from(e)))?;
          if existing.iter().any(|change| changes.contains(change)) {
            return Err(ConflictableTransactionError::Abort(StoreError::Conflict(
              "Change already committed".to_string(),
            )));
          }
          existing.extend_from_slice(changes);
          existing
        }
        None => changes.to_vec(),
      };
      let raw =
        rmp_serde::to_vec(&new_changes).map_err(|e| ConflictableTransactionError::Abort(StoreError::from(e)))?;
      tx.insert(self.node_id.as_str(), raw)?;
      Ok(())
    })?;
    Ok(())
  }
}
