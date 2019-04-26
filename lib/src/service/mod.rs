use std::sync::Arc;

mod config;
mod error;
mod local;

pub use self::config::{config_file, StoreConfig};
pub use self::error::*;

use crate::secrets_store::SecretsStore;

pub trait TrustlessService {
  fn list_stores(&self) -> ServiceResult<Vec<String>>;

  fn set_store_config(&self, store_config: StoreConfig) -> ServiceResult<()>;

  fn get_store_config(&self, name: &str) -> ServiceResult<StoreConfig>;

  fn open_store(&self, name: &str) -> ServiceResult<Arc<SecretsStore>>;

  fn get_default_store(&self) -> ServiceResult<Option<String>>;

  fn set_default_store(&self, name: &str) -> ServiceResult<()>;
}

pub fn create_service() -> ServiceResult<Arc<TrustlessService>> {
  // TODO: Some magic to figure out if we are remote or local (atm there is only local anyway
  Ok(Arc::new(self::local::LocalTrustlessService::new()?))
}
