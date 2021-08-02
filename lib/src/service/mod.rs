use crate::api::{EventHandler, EventSubscription, PasswordGeneratorParam};
use std::sync::Arc;

mod config;
mod error;
pub mod local;
pub mod pw_generator;
mod remote;
mod secrets_provider;

#[cfg(unix)]
pub mod unix;

pub use self::config::{config_file, StoreConfig};
pub use self::error::*;

use crate::secrets_store::SecretsStore;

pub trait ClipboardControl: Send + Sync {
  fn is_done(&self) -> ServiceResult<bool>;

  fn currently_providing(&self) -> ServiceResult<Option<String>>;

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
  fn open_store(&self, name: &str) -> ServiceResult<Arc<dyn SecretsStore>>;

  /// Get the name of the store that should be opened by default
  fn get_default_store(&self) -> ServiceResult<Option<String>>;

  /// Set the name of the store that should be opened by default
  fn set_default_store(&self, name: &str) -> ServiceResult<()>;

  fn secret_to_clipboard(
    &self,
    store_name: &str,
    block_id: &str,
    properties: &[&str],
    display_name: &str,
  ) -> ServiceResult<Arc<dyn ClipboardControl>>;

  fn add_event_handler(&self, handler: Box<dyn EventHandler>) -> ServiceResult<Box<dyn EventSubscription>>;

  fn generate_id(&self) -> ServiceResult<String>;

  fn generate_password(&self, param: PasswordGeneratorParam) -> ServiceResult<String>;

  fn check_autolock(&self);
}

pub fn create_service() -> ServiceResult<Arc<dyn TrustlessService>> {
  #[cfg(unix)]
  {
    if let Some(remote) = self::unix::try_remote_service()? {
      return Ok(Arc::new(remote));
    }
  }
  Ok(Arc::new(self::local::LocalTrustlessService::new()?))
}
