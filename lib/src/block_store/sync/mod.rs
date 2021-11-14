use std::sync::{Arc, Mutex};

use crate::memguard::weak::ZeroingWords;

use super::{BlockStore, ChangeLog, RingContent, RingId, StoreError, StoreResult};

mod synchronize;

#[cfg(test)]
mod synchronize_tests;

pub struct SyncBlockStore {
  local: Arc<dyn BlockStore>,
  remote: Arc<dyn BlockStore>,
  sync_lock: Arc<Mutex<()>>,
}

impl SyncBlockStore {
  pub fn new(local: Arc<dyn BlockStore>, remote: Arc<dyn BlockStore>) -> SyncBlockStore {
    SyncBlockStore {
      local,
      remote,
      sync_lock: Arc::new(Mutex::new(())),
    }
  }

  pub fn synchronize(&self) -> StoreResult<()> {
    let _guard = self.sync_lock.lock()?;

    synchronize::synchronize_rings(self.local.clone(), self.remote.clone())?;
    synchronize::synchronize_blocks(self.local.clone(), self.remote.clone())
  }
}

impl BlockStore for SyncBlockStore {
  fn node_id(&self) -> &str {
    self.local.node_id()
  }

  fn list_ring_ids(&self) -> StoreResult<Vec<RingId>> {
    self.local.list_ring_ids()
  }

  fn get_ring(&self, ring_id: &str) -> StoreResult<RingContent> {
    match self.local.get_ring(ring_id) {
      Ok(ring) => Ok(ring),
      Err(StoreError::InvalidBlock(_)) => self.remote.get_ring(ring_id),
      Err(err) => Err(err),
    }
  }

  fn store_ring(&self, ring_id: &str, version: u64, raw: &[u8]) -> StoreResult<()> {
    self.local.store_ring(ring_id, version, raw)
  }

  fn change_logs(&self) -> StoreResult<Vec<super::ChangeLog>> {
    self.local.change_logs()
  }

  fn get_index(&self, index_id: &str) -> StoreResult<Option<ZeroingWords>> {
    self.local.get_index(index_id)
  }

  fn store_index(&self, index_id: &str, raw: &[u8]) -> StoreResult<()> {
    self.local.store_index(index_id, raw)
  }

  fn add_block(&self, raw: &[u8]) -> StoreResult<String> {
    self.local.add_block(raw)
  }

  fn get_block(&self, block: &str) -> StoreResult<ZeroingWords> {
    match self.local.get_block(block) {
      Ok(content) => Ok(content),
      Err(StoreError::InvalidBlock(_)) => self.remote.get_block(block),
      Err(err) => Err(err),
    }
  }

  fn commit(&self, changes: &[super::Change]) -> StoreResult<()> {
    self.local.commit(changes)
  }

  fn update_change_log(&self, _change_log: ChangeLog) -> StoreResult<()> {
    // Note: Intentionally left blank. There should be no nested sync stores
    Ok(())
  }
}
