use std::collections::HashMap;
use std::sync::RwLock;

use super::{generate_block_id, BlockStore, Change, ChangeLog, StoreError, StoreResult};
use crate::memguard::weak::ZeroingWords;

/// Memory based reference implementation of a block store.
///
/// This is mostly useful for unit-testing, but might have some other usa-cases in
/// the future.
///
pub struct MemoryBlockStore {
  node_id: String,
  rings: RwLock<HashMap<String, ZeroingWords>>,
  indexes: RwLock<HashMap<String, ZeroingWords>>,
  blocks: RwLock<HashMap<String, ZeroingWords>>,
  changes: RwLock<HashMap<String, Vec<Change>>>,
}

impl MemoryBlockStore {
  pub fn new(node_id: &str) -> MemoryBlockStore {
    MemoryBlockStore {
      node_id: node_id.to_string(),
      rings: RwLock::new(HashMap::new()),
      indexes: RwLock::new(HashMap::new()),
      blocks: RwLock::new(HashMap::new()),
      changes: RwLock::new(HashMap::new()),
    }
  }
}

impl BlockStore for MemoryBlockStore {
  fn node_id(&self) -> &str {
    &self.node_id
  }

  fn list_ring_ids(&self) -> StoreResult<Vec<String>> {
    let rings = self.rings.read()?;

    Ok(rings.keys().cloned().collect())
  }

  fn get_ring(&self, ring_id: &str) -> StoreResult<ZeroingWords> {
    let rings = self.rings.read()?;

    rings
      .get(ring_id)
      .cloned()
      .ok_or_else(|| StoreError::InvalidBlock(ring_id.to_string()))
  }

  fn store_ring(&self, ring_id: &str, raw: &[u8]) -> StoreResult<()> {
    let mut rings = self.rings.write()?;

    rings.insert(ring_id.to_string(), raw.into());

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

  fn get_index(&self, node: &str) -> StoreResult<Option<ZeroingWords>> {
    let indexes = self.indexes.read()?;

    Ok(indexes.get(node).cloned())
  }

  fn store_index(&self, node: &str, raw: &[u8]) -> StoreResult<()> {
    let mut indexes = self.indexes.write()?;

    indexes.insert(node.to_string(), raw.into());
    Ok(())
  }

  fn add_block(&self, raw: &[u8]) -> StoreResult<String> {
    let block_id = generate_block_id(raw);
    let mut blocks = self.blocks.write()?;

    blocks.insert(block_id.clone(), raw.into());
    Ok(block_id)
  }

  fn get_block(&self, block: &str) -> StoreResult<ZeroingWords> {
    let blocks = self.blocks.read()?;

    blocks
      .get(block)
      .cloned()
      .ok_or_else(|| StoreError::InvalidBlock(block.to_string()))
  }

  fn commit(&self, changes: &[Change]) -> StoreResult<()> {
    let mut stored_changes = self.changes.write()?;

    match stored_changes.get_mut(&self.node_id) {
      Some(existing) => {
        if existing.iter().any(|change| changes.contains(change)) {
          return Err(StoreError::Conflict("Change already committed".to_string()));
        }
        existing.extend_from_slice(changes);
      }
      None => {
        stored_changes.insert(self.node_id.to_string(), changes.to_vec());
      }
    }
    Ok(())
  }
}
