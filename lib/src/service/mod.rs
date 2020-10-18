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

pub trait ClipboardControl {
  fn is_done(&self) -> ServiceResult<bool>;

  fn currently_providing(&self) -> ServiceResult<Option<String>>;

  fn provide_next(&self) -> ServiceResult<()>;

  fn destroy(&self) -> ServiceResult<()>;
}

pub trait TrustlessService: std::fmt::Debug {
  fn list_stores(&self) -> ServiceResult<Vec<String>>;

  fn set_store_config(&self, store_config: StoreConfig) -> ServiceResult<()>;

  fn get_store_config(&self, name: &str) -> ServiceResult<StoreConfig>;

  fn open_store(&self, name: &str) -> ServiceResult<Arc<dyn SecretsStore>>;

  fn get_default_store(&self) -> ServiceResult<Option<String>>;

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
