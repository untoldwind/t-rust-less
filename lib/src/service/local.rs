use crate::secrets_store::{open_secrets_store, SecretsStore};
use crate::service::config::{read_config, write_config, Config};
use crate::service::error::{ServiceError, ServiceResult};
use crate::service::{StoreConfig, TrustlessService};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub struct LocalTrustlessService {
  config: RwLock<Config>,
  opened_stores: RwLock<HashMap<String, Arc<SecretsStore>>>,
}

impl LocalTrustlessService {
  pub fn new() -> ServiceResult<LocalTrustlessService> {
    let config = read_config()?.unwrap_or_default();

    Ok(LocalTrustlessService {
      config: RwLock::new(config),
      opened_stores: RwLock::new(HashMap::new()),
    })
  }
}

impl TrustlessService for LocalTrustlessService {
  fn list_stores(&self) -> ServiceResult<Vec<String>> {
    let config = self.config.read()?;

    Ok(config.stores.keys().cloned().collect())
  }

  fn set_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    let mut config = self.config.write()?;

    if config.default_store.is_none() {
      config.default_store = Some(store_config.name.to_string());
    }
    config.stores.insert(store_config.name.to_string(), store_config);
    write_config(&config)?;

    Ok(())
  }

  fn get_store_config(&self, name: &str) -> ServiceResult<StoreConfig> {
    let config = self.config.read()?;

    Ok(
      config
        .stores
        .get(name)
        .cloned()
        .ok_or_else(|| ServiceError::StoreNotFound(name.to_string()))?,
    )
  }

  fn open_store(&self, name: &str) -> ServiceResult<Arc<SecretsStore>> {
    {
      let opened_stores = self.opened_stores.read()?;

      if let Some(store) = opened_stores.get(name) {
        return Ok(store.clone());
      }
    }
    let mut opened_stores = self.opened_stores.write()?;
    let config = self.config.read()?;
    let store_config = config
      .stores
      .get(name)
      .ok_or_else(|| ServiceError::StoreNotFound(name.to_string()))?;
    let store = open_secrets_store(
      &store_config.store_url,
      &store_config.client_id,
  Duration::from_secs(    store_config.autolock_timeout_secs),
    )?;

    opened_stores.insert(name.to_string(), store.clone());

    Ok(store)
  }

  fn get_default_store(&self) -> ServiceResult<Option<String>> {
    let config = self.config.read()?;

    Ok(config.default_store.to_owned())
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    let mut config = self.config.write()?;

    if !config.stores.contains_key(name) {
      return Err(ServiceError::StoreNotFound(name.to_string()));
    }

    config.default_store = Some(name.to_string());
    write_config(&config)?;

    Ok(())
  }
}
