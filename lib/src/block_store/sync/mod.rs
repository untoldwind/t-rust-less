use std::{
  sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
  },
  thread,
  thread::JoinHandle,
  time::Duration,
};

use log::{error, info};

use crate::memguard::weak::ZeroingWords;

use super::{BlockStore, ChangeLog, RingContent, RingId, StoreError, StoreResult};

mod synchronize;

#[cfg(test)]
mod synchronize_tests;

pub struct SyncBlockStore {
  local: Arc<dyn BlockStore>,
  remote: Arc<dyn BlockStore>,
  sync_interval: Duration,
  sync_lock: Arc<Mutex<()>>,
  worker: Option<(JoinHandle<()>, Arc<AtomicBool>)>,
}

impl SyncBlockStore {
  pub fn new(local: Arc<dyn BlockStore>, remote: Arc<dyn BlockStore>, sync_interval: Duration) -> SyncBlockStore {
    SyncBlockStore {
      local,
      remote,
      sync_interval,
      sync_lock: Arc::new(Mutex::new(())),
      worker: None,
    }
  }

  pub fn synchronize(&self) -> StoreResult<()> {
    let _guard = self.sync_lock.lock()?;

    synchronize::synchronize_rings(self.local.clone(), self.remote.clone())?;
    synchronize::synchronize_blocks(self.local.clone(), self.remote.clone())
  }

  pub fn start_worker(&mut self) {
    self.stop_worker();

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_flag_cloned = stop_flag.clone();
    let local = self.local.clone();
    let remote = self.remote.clone();
    let sync_lock = self.sync_lock.clone();
    let sync_interval = self.sync_interval;
    let join_handle = thread::spawn(move || loop {
      if stop_flag_cloned.load(Ordering::Relaxed) {
        info!("Synchronization worker stopping");
        return;
      }
      match sync_lock.lock() {
        Ok(_guard) => {
          if let Err(err) = synchronize::synchronize_rings(local.clone(), remote.clone()) {
            error!("Store synchronization failed: {}", err);
            continue;
          }
          if let Err(err) = synchronize::synchronize_blocks(local.clone(), remote.clone()) {
            error!("Store synchronization failed: {}", err);
          }
        }
        Err(err) => {
          error!("Obtain synchronization lock failed: {}", err)
        }
      };

      thread::sleep(sync_interval)
    });
    self.worker = Some((join_handle, stop_flag));
  }

  fn stop_worker(&mut self) {
    if let Some((join_handle, stop_flag)) = self.worker.take() {
      stop_flag.store(true, Ordering::Relaxed);
      join_handle.join().ok();
    }
  }
}

impl Drop for SyncBlockStore {
  fn drop(&mut self) {
    self.stop_worker();
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
