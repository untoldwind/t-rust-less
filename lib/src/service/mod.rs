use chrono::{DateTime, Utc};

use crate::api::{ClipboardProviding, Event, PasswordGeneratorParam, StoreConfig};
use std::sync::Arc;

mod config;
mod error;
pub mod local;
pub mod pw_generator;
mod remote;
pub mod secrets_provider;
mod synchronizer;

#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

pub use self::config::config_file;
pub use self::error::*;

use crate::secrets_store::{SecretStoreResult, SecretsStore};

pub trait ClipboardControl: Send + Sync {
  fn is_done(&self) -> ServiceResult<bool>;

  fn currently_providing(&self) -> ServiceResult<Option<ClipboardProviding>>;

  fn provide_next(&self) -> ServiceResult<()>;

  fn destroy(&self) -> ServiceResult<()>;
}

/// Main entrypoint for all interactions with the t-rust-less system
pub trait TrustlessService: std::fmt::Debug + Send + Sync {
  /// List all store configurations
  fn list_stores(&self) -> ServiceResult<Vec<StoreConfig>>;

  /// Create or update a store configuration
  fn upsert_store_config(&self, store_config: StoreConfig) -> ServiceResult<()>;

  /// Delete a store configuration
  /// (This will only delete the configuration, the store itself will be left untouched)
  fn delete_store_config(&self, name: &str) -> ServiceResult<()>;

  /// Open a store
  fn open_store(&self, name: &str) -> SecretStoreResult<Arc<dyn SecretsStore>>;

  /// Get the name of the store that should be opened by default
  fn get_default_store(&self) -> ServiceResult<Option<String>>;

  /// Set the name of the store that should be opened by default
  fn set_default_store(&self, name: &str) -> ServiceResult<()>;

  fn secret_to_clipboard(
    &self,
    store_name: &str,
    block_id: &str,
    properties: &[&str],
  ) -> ServiceResult<Arc<dyn ClipboardControl>>;

  fn poll_events(&self, last_id: u64) -> ServiceResult<Vec<Event>>;

  fn generate_id(&self) -> ServiceResult<String>;

  fn generate_password(&self, param: PasswordGeneratorParam) -> ServiceResult<String>;

  fn check_autolock(&self);

  fn needs_synchronization(&self) -> bool;

  fn synchronize(&self) -> Option<DateTime<Utc>>;
}

pub fn create_service() -> ServiceResult<Arc<dyn TrustlessService>> {
  #[cfg(unix)]
  {
    if let Some(remote) = self::unix::try_remote_service()? {
      return Ok(Arc::new(remote));
    }
  }
  #[cfg(windows)]
  {
    if let Some(remote) = self::windows::try_remote_service()? {
      return Ok(Arc::new(remote));
    }
  }
  Ok(Arc::new(self::local::LocalTrustlessService::new()?))
}
