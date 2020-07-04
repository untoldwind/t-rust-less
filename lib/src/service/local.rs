use super::pw_generator::{generate_chars, generate_words};
use crate::api::{Event, EventHandler, EventHub, EventSubscription, PasswordGeneratorParam};
use crate::clipboard::Clipboard;
use crate::secrets_store::{open_secrets_store, SecretsStore};
use crate::service::config::{read_config, write_config, Config};
use crate::service::error::{ServiceError, ServiceResult};
#[cfg(unix)]
use crate::service::secrets_provider::SecretsProvider;
use crate::service::{ClipboardControl, StoreConfig, TrustlessService};
use chrono::Utc;
use log::{error, info};
use rand::{distributions, thread_rng, Rng};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

enum ClipboardHolder {
  Empty,
  Providing(Clipboard),
}

impl ClipboardControl for ClipboardHolder {
  fn is_done(&self) -> ServiceResult<bool> {
    match self {
      ClipboardHolder::Empty => Ok(true),
      ClipboardHolder::Providing(clipboard) => Ok(!clipboard.is_open()),
    }
  }

  fn currently_providing(&self) -> ServiceResult<Option<String>> {
    match self {
      ClipboardHolder::Empty => Ok(None),
      ClipboardHolder::Providing(clipboard) => Ok(clipboard.currently_providing()),
    }
  }

  fn destroy(&self) -> ServiceResult<()> {
    if let ClipboardHolder::Providing(clipboard) = &self {
      clipboard.destroy();
    }
    Ok(())
  }
}

struct LocalEventHubInner {
  next_id: u32,
  handlers: HashMap<u32, Box<dyn EventHandler>>,
}

struct LocalEventHub {
  inner: RwLock<LocalEventHubInner>,
}

impl LocalEventHub {
  fn new() -> LocalEventHub {
    LocalEventHub {
      inner: RwLock::new(LocalEventHubInner {
        next_id: 0,
        handlers: HashMap::new(),
      }),
    }
  }

  fn add_event_handler(&self, handler: Box<dyn EventHandler>) -> ServiceResult<u32> {
    let mut inner = self.inner.write()?;
    let id = inner.next_id;

    inner.handlers.insert(id, handler);
    inner.next_id += 1;

    Ok(id)
  }

  fn remove_event_handler(&self, id: u32) {
    match self.inner.write() {
      Ok(mut inner) => {
        inner.handlers.remove(&id);
      }
      Err(err) => {
        error!("Unregister event handler failed: {}", err);
      }
    }
  }
}

impl EventHub for LocalEventHub {
  fn send(&self, event: Event) {
    match self.inner.read() {
      Ok(inner) => {
        for event_handler in inner.handlers.values() {
          event_handler.handle(event.clone());
        }
      }
      Err(e) => {
        error!("Queue event failed: {}", e);
      }
    };
  }
}

struct LocalEventSubscription {
  event_hub: Arc<LocalEventHub>,
  id: u32,
}

impl LocalEventSubscription {
  fn new(event_hub: Arc<LocalEventHub>, id: u32) -> LocalEventSubscription {
    LocalEventSubscription { event_hub, id }
  }
}

impl Drop for LocalEventSubscription {
  fn drop(&mut self) {
    self.event_hub.remove_event_handler(self.id)
  }
}

impl EventSubscription for LocalEventSubscription {}

pub struct LocalTrustlessService {
  config: RwLock<Config>,
  opened_stores: RwLock<HashMap<String, Arc<dyn SecretsStore>>>,
  clipboard: RwLock<Arc<ClipboardHolder>>,
  event_hub: Arc<LocalEventHub>,
}

impl LocalTrustlessService {
  pub fn new() -> ServiceResult<LocalTrustlessService> {
    let config = read_config()?.unwrap_or_default();

    Ok(LocalTrustlessService {
      config: RwLock::new(config),
      opened_stores: RwLock::new(HashMap::new()),
      clipboard: RwLock::new(Arc::new(ClipboardHolder::Empty)),
      event_hub: Arc::new(LocalEventHub::new()),
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

  fn open_store(&self, name: &str) -> ServiceResult<Arc<dyn SecretsStore>> {
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
      name,
      &store_config.store_url,
      &store_config.client_id,
      Duration::from_secs(store_config.autolock_timeout_secs),
      self.event_hub.clone(),
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

  fn secret_to_clipboard(
    &self,
    store_name: &str,
    secret_id: &str,
    properties: &[&str],
    display_name: &str,
  ) -> ServiceResult<Arc<dyn ClipboardControl>> {
    #[cfg(unix)]
    {
      let store = self.open_store(store_name)?;
      let secret = store.get(secret_id)?;
      let secret_provider = SecretsProvider::new(secret.current, properties);
      let mut clipboard = self.clipboard.write()?;

      info!("Providing {} for {} in {}", properties.join(","), secret_id, store_name);

      let next_clipboard = Arc::new(ClipboardHolder::Providing(Clipboard::new(
        display_name,
        secret_provider,
        store_name.to_string(),
        secret_id.to_string(),
        self.event_hub.clone(),
      )?));
      *clipboard = next_clipboard.clone();

      Ok(next_clipboard)
    }
    #[cfg(not(unix))]
    {
      Err(ServiceError::NotAvailable)
    }
  }

  fn add_event_handler(&self, handler: Box<dyn EventHandler>) -> ServiceResult<Box<dyn EventSubscription>> {
    let id = self.event_hub.add_event_handler(handler)?;

    Ok(Box::new(LocalEventSubscription::new(self.event_hub.clone(), id)))
  }

  fn generate_id(&self) -> ServiceResult<String> {
    let rng = thread_rng();

    Ok(
      rng
        .sample_iter(&distributions::Alphanumeric)
        .take(64)
        .collect::<String>(),
    )
  }

  fn generate_password(&self, param: PasswordGeneratorParam) -> ServiceResult<String> {
    match param {
      PasswordGeneratorParam::Chars(params) => Ok(generate_chars(params)),
      PasswordGeneratorParam::Words(params) => Ok(generate_words(params)),
    }
  }
}
