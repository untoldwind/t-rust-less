use crate::clipboard::Clipboard;
use crate::secrets_store::{open_secrets_store, SecretsStore};
use crate::service::config::{read_config, write_config, Config};
use crate::service::error::{ServiceError, ServiceResult};
use crate::service::secrets_provider::SecretsProvider;
use crate::service::{StoreConfig, TrustlessService};
use chrono::Utc;
use log::{error, info};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

pub struct LocalTrustlessService {
  config: RwLock<Config>,
  opened_stores: RwLock<HashMap<String, Arc<SecretsStore>>>,
  clipboard: RwLock<Option<Clipboard>>,
}

impl LocalTrustlessService {
  pub fn new() -> ServiceResult<LocalTrustlessService> {
    let config = read_config()?.unwrap_or_default();

    Ok(LocalTrustlessService {
      config: RwLock::new(config),
      opened_stores: RwLock::new(HashMap::new()),
      clipboard: RwLock::new(None),
    })
  }

  pub fn check_autolock(&self) {
    let opened_stores = match self.opened_stores.read() {
      Ok(opened_stores) => opened_stores,
      Err(err) => {
        error!("Failed locking opened stores: {}", err);
        return;
      }
    };

    for (name, secrets_store) in opened_stores.iter() {
      let status = match secrets_store.status() {
        Ok(status) => status,
        Err(error) => {
          error!("Autolocker was unable to query status: {}", error);
          continue;
        }
      };

      if let Some(autolock_at) = status.autolock_at {
        if autolock_at < Utc::now() {
          info!("Autolocking {}", name);
          if let Err(error) = secrets_store.lock() {
            error!("Autolocker was unable to lock store: {}", error);
          }
        }
      }
    }
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
      Duration::from_secs(store_config.autolock_timeout_secs),
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

  fn direct_clipboard_available(&self) -> ServiceResult<bool> {
    #[cfg(unix)]
    {
      Ok(true)
    }
    #[cfg(not(unix))]
    {
      Ok(false)
    }
  }

  fn secret_to_clipboard(&self, store_name: &str, secret_id: &str, properties: &[&str]) -> ServiceResult<()> {
    let store = self.open_store(store_name)?;
    let secret = store.get(secret_id)?;
    let secret_provider = SecretsProvider::new(secret.current.clone(), properties);
    let mut clipboard = self.clipboard.write()?;

    info!("Providing {} for {} in {}", properties.join(","), secret_id, store_name);

    clipboard.replace(Clipboard::new(secret_provider)?);

    Ok(())
  }
}
