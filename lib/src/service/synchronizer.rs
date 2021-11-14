use std::sync::Arc;

use crate::{block_store::sync::SyncBlockStore, secrets_store::SecretsStore};

use super::ServiceResult;

#[derive(Debug)]
pub struct Synchronizer {
  secret_store: Arc<dyn SecretsStore>,
  sync_block_store: Arc<SyncBlockStore>,
}

impl Synchronizer {
  pub fn new(secret_store: Arc<dyn SecretsStore>, sync_block_store: Arc<SyncBlockStore>) -> Self {
    Synchronizer {
      secret_store,
      sync_block_store,
    }
  }

  pub fn synchronize(&self) -> ServiceResult<()> {
    self.sync_block_store.synchronize()?;

    if !self.secret_store.status()?.locked {
      self.secret_store.update_index()?;
    }

    Ok(())
  }
}
