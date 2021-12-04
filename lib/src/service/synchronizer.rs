use chrono::{DateTime, Duration, Utc};
use log::info;
use std::sync::Arc;

use crate::{block_store::sync::SyncBlockStore, secrets_store::SecretsStore};

use super::ServiceResult;

#[derive(Debug)]
pub struct Synchronizer {
  secret_store: Arc<dyn SecretsStore>,
  sync_block_store: Arc<SyncBlockStore>,
  sync_interval: Duration,
  last_run: Option<DateTime<Utc>>,
}

impl Synchronizer {
  pub fn new(
    secret_store: Arc<dyn SecretsStore>,
    sync_block_store: Arc<SyncBlockStore>,
    sync_interval: Duration,
  ) -> Self {
    Synchronizer {
      secret_store,
      sync_block_store,
      sync_interval,
      last_run: None,
    }
  }

  pub fn synchronize(&mut self) -> ServiceResult<()> {
    if let Some(last_run) = self.last_run {
      if last_run + self.sync_interval > Utc::now() {
        return Ok(());
      }
    }
    info!("Start store synchronization");
    self.last_run = Some(Utc::now());

    let local_changes = self.sync_block_store.synchronize()?;

    if local_changes && !self.secret_store.status()?.locked {
      self.secret_store.update_index()?;
    }

    Ok(())
  }

  pub fn next_run(&self) -> DateTime<Utc> {
    match self.last_run {
      Some(last_run) => last_run + self.sync_interval,
      None => Utc::now(),
    }
  }
}
