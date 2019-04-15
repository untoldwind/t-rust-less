use std::collections::HashMap;
use std::sync::RwLock;

use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};

use super::{BlockStore, Change, ChangeLog, StoreError, StoreResult};

/// Memory based reference implementation of a block store.
///
/// This is mostly useful for unit-testing, but might have some other usa-cases in
/// the future.
///
pub struct MemoryBlockStore {
  ring: RwLock<Option<Vec<u8>>>,
  pub_ring: RwLock<Option<Vec<u8>>>,
  indexes: RwLock<HashMap<String, Vec<u8>>>,
  blocks: RwLock<HashMap<String, Vec<u8>>>,
  changes: RwLock<HashMap<String, Vec<Change>>>,
}

impl MemoryBlockStore {
  pub fn new() -> MemoryBlockStore {
    MemoryBlockStore {
      ring: RwLock::new(None),
      pub_ring: RwLock::new(None),
      indexes: RwLock::new(HashMap::new()),
      blocks: RwLock::new(HashMap::new()),
      changes: RwLock::new(HashMap::new()),
    }
  }

  fn generate_id(data: &[u8]) -> String {
    let mut hasher = Sha256::new();

    hasher.input(data);

    HEXLOWER.encode(&hasher.result())
  }
}

impl BlockStore for MemoryBlockStore {
  fn get_ring(&self) -> StoreResult<Option<Vec<u8>>> {
    let ring = self.ring.read()?;

    Ok(ring.clone())
  }

  fn store_ring(&self, raw: &[u8]) -> StoreResult<()> {
    let mut ring = self.ring.write()?;

    ring.replace(raw.to_vec());
    Ok(())
  }

  fn get_public_ring(&self) -> StoreResult<Option<Vec<u8>>> {
    let pub_ring = self.pub_ring.read()?;

    Ok(pub_ring.clone())
  }

  fn store_public_ring(&self, raw: &[u8]) -> StoreResult<()> {
    let mut pub_ring = self.pub_ring.write()?;

    pub_ring.replace(raw.to_vec());
    Ok(())
  }

  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>> {
    let changes = self.changes.read()?;

    Ok(
      changes
        .iter()
        .map(|(node, changes)| ChangeLog {
          node: node.clone(),
          changes: changes.clone(),
        })
        .collect(),
    )
  }

  fn get_index(&self, node: &str) -> StoreResult<Option<Vec<u8>>> {
    let indexes = self.indexes.read()?;

    Ok(indexes.get(node).cloned())
  }

  fn store_index(&self, node: &str, raw: &[u8]) -> StoreResult<()> {
    let mut indexes = self.indexes.write()?;

    indexes.insert(node.to_string(), raw.to_vec());
    Ok(())
  }

  fn add_block(&self, raw: &[u8]) -> StoreResult<String> {
    let block_id = Self::generate_id(raw);
    let mut blocks = self.blocks.write()?;

    blocks.insert(block_id.clone(), raw.to_vec());
    Ok(block_id)
  }

  fn get_block(&self, block: &str) -> StoreResult<Vec<u8>> {
    let blocks = self.blocks.read()?;

    blocks
      .get(block)
      .cloned()
      .ok_or_else(|| StoreError::InvalidBlock(block.to_string()))
  }

  fn commit(&self, node: &str, changes: &[Change]) -> StoreResult<()> {
    let mut stored_changes = self.changes.write()?;

    match stored_changes.get_mut(node) {
      Some(existing) => {
        existing.extend_from_slice(changes);
      }
      None => {
        stored_changes.insert(node.to_string(), changes.to_vec());
      }
    }
    Ok(())
  }
}
