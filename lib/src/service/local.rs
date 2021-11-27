use super::pw_generator::{generate_chars, generate_words};
use super::synchronizer::Synchronizer;
use crate::api::{ClipboardProviding, Event, EventData, EventHub, PasswordGeneratorParam, StoreConfig};
use crate::block_store::StoreError;
use crate::clipboard::{Clipboard, ClipboardCommon};
use crate::secrets_store::{open_secrets_store, SecretStoreResult, SecretsStore};
use crate::service::config::{read_config, write_config, Config};
use crate::service::error::{ServiceError, ServiceResult};
#[cfg(any(unix, windows))]
use crate::service::secrets_provider::SecretsProvider;
use crate::service::{ClipboardControl, TrustlessService};
use chrono::{DateTime, Utc};
use log::{error, info};
use rand::{distributions, thread_rng, Rng};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
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

  fn currently_providing(&self) -> ServiceResult<Option<ClipboardProviding>> {
    match self {
      ClipboardHolder::Empty => Ok(None),
      ClipboardHolder::Providing(clipboard) => Ok(clipboard.currently_providing()),
    }
  }

  fn provide_next(&self) -> ServiceResult<()> {
    if let ClipboardHolder::Providing(clipboard) = &self {
      clipboard.provide_next();
    }
    Ok(())
  }

  fn destroy(&self) -> ServiceResult<()> {
    if let ClipboardHolder::Providing(clipboard) = &self {
      clipboard.destroy();
    }
    Ok(())
  }
}

struct LocalEventQueue {
  last_id: u64,
  limit: usize,
  queue: VecDeque<Event>,
}

impl LocalEventQueue {
  fn new(limit: usize) -> Self {
    LocalEventQueue {
      last_id: 0,
      limit,
      queue: VecDeque::with_capacity(limit),
    }
  }

  fn queue(&mut self, data: EventData) {
    if self.queue.len() >= self.limit {
      self.queue.pop_front();
    }
    self.last_id += 1;
    self.queue.push_back(Event { id: self.last_id, data });
  }

  fn poll(&self, last_id: u64) -> Vec<Event> {
    match self.queue.iter().position(|e| e.id > last_id) {
      Some(start) => self.queue.iter().skip(start).cloned().collect(),
      None => vec![],
    }
  }
}

struct LocalEventHub {
  event_queue: RwLock<LocalEventQueue>,
}

impl LocalEventHub {
  fn new(limit: usize) -> Self {
    LocalEventHub {
      event_queue: RwLock::new(LocalEventQueue::new(limit)),
    }
  }

  fn poll_events(&self, last_id: u64) -> ServiceResult<Vec<Event>> {
    let event_queue = self.event_queue.read()?;

    Ok(event_queue.poll(last_id))
  }
}

impl EventHub for LocalEventHub {
  fn send(&self, event: EventData) {
    match self.event_queue.write() {
      Ok(mut event_queue) => event_queue.queue(event),
      Err(e) => {
        error!("Queue event failed: {}", e);
      }
    };
  }
}

pub struct LocalTrustlessService {
  config: RwLock<Config>,
  opened_stores: RwLock<HashMap<String, Arc<dyn SecretsStore>>>,
  synchronizers: Mutex<Vec<Synchronizer>>,
  clipboard: RwLock<Arc<ClipboardHolder>>,
  event_hub: Arc<LocalEventHub>,
}

impl LocalTrustlessService {
  pub fn new() -> ServiceResult<LocalTrustlessService> {
    let config = read_config()?.unwrap_or_default();

    Ok(LocalTrustlessService {
      config: RwLock::new(config),
      opened_stores: RwLock::new(HashMap::new()),
      synchronizers: Mutex::new(vec![]),
      clipboard: RwLock::new(Arc::new(ClipboardHolder::Empty)),
      event_hub: Arc::new(LocalEventHub::new(100)),
    })
  }
}

impl TrustlessService for LocalTrustlessService {
  fn list_stores(&self) -> ServiceResult<Vec<StoreConfig>> {
    let config = self.config.read()?;

    Ok(config.stores.values().cloned().collect())
  }

  fn upsert_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    let mut config = self.config.write()?;

    if config.default_store.is_none() {
      config.default_store = Some(store_config.name.to_string());
    }
    config.stores.insert(store_config.name.to_string(), store_config);
    write_config(&config)?;

    Ok(())
  }

  fn delete_store_config(&self, name: &str) -> ServiceResult<()> {
    let mut config = self.config.write()?;

    if config.stores.remove(name).is_some() {
      write_config(&config)?;
    }

    Ok(())
  }

  fn open_store(&self, name: &str) -> SecretStoreResult<Arc<dyn SecretsStore>> {
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
      .ok_or_else(|| StoreError::StoreNotFound(name.to_string()))?;
    let (store, maybe_sync_block_store) = open_secrets_store(
      name,
      &store_config.store_url,
      store_config.remote_url.as_deref(),
      &store_config.client_id,
      Duration::from_secs(store_config.autolock_timeout_secs),
      self.event_hub.clone(),
    )?;

    if let Some(sync_block_store) = maybe_sync_block_store {
      self.synchronizers.lock()?.push(Synchronizer::new(
        store.clone(),
        sync_block_store,
        chrono::Duration::seconds(store_config.sync_interval_sec as i64),
      ));
    }

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
    block_id: &str,
    properties: &[&str],
  ) -> ServiceResult<Arc<dyn ClipboardControl>> {
    #[cfg(any(unix, windows))]
    {
      let store = self.open_store(store_name)?;
      let secret_version = store.get_version(block_id)?;
      let secret_provider =
        SecretsProvider::new(store_name.to_string(), block_id.to_string(), secret_version, properties);
      let mut clipboard = self.clipboard.write()?;

      clipboard.destroy()?;

      info!("Providing {} for {} in {}", properties.join(","), block_id, store_name);

      let next_clipboard = Arc::new(ClipboardHolder::Providing(Clipboard::new(
        secret_provider,
        self.event_hub.clone(),
      )?));
      *clipboard = next_clipboard.clone();

      Ok(next_clipboard)
    }
    #[cfg(not(any(unix, windows)))]
    {
      Err(ServiceError::NotAvailable)
    }
  }

  fn poll_events(&self, last_id: u64) -> ServiceResult<Vec<Event>> {
    self.event_hub.poll_events(last_id)
  }

  fn generate_id(&self) -> ServiceResult<String> {
    let rng = thread_rng();

    Ok(
      rng
        .sample_iter(distributions::Alphanumeric)
        .map(char::from)
        .take(64)
        .collect::<String>(),
    )
  }

  fn generate_password(&self, param: PasswordGeneratorParam) -> ServiceResult<String> {
    match &param {
      PasswordGeneratorParam::Chars(params) => Ok(generate_chars(params)),
      PasswordGeneratorParam::Words(params) => Ok(generate_words(params)),
    }
  }

  fn check_autolock(&self) {
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
        if autolock_at < Utc::now().into() {
          info!("Autolocking {}", name);
          if let Err(error) = secrets_store.lock() {
            error!("Autolocker was unable to lock store: {}", error);
          }
        }
      }
    }
  }

  fn needs_synchronization(&self) -> bool {
    if let Ok(config) = self.config.read() {
      config
        .stores
        .iter()
        .any(|(_, store_config)| store_config.remote_url.is_some())
    } else {
      false
    }
  }

  fn synchronize(&self) -> Option<DateTime<Utc>> {
    match self.synchronizers.lock() {
      Ok(mut synchronizers) => {
        let mut result = None;
        for synchronizer in synchronizers.iter_mut() {
          if let Err(err) = synchronizer.synchronize() {
            error!("Synchronization failed: {}", err);
          }
          let next = synchronizer.next_run();
          result = match result {
            Some(prev) if prev > next => Some(next),
            Some(prev) => Some(prev),
            None => Some(next),
          };
        }
        result
      }
      Err(err) => {
        error!("Synchronization lock failed: {}", err);
        None
      }
    }
  }
}

impl std::fmt::Debug for LocalTrustlessService {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Local Trustless service")
  }
}
