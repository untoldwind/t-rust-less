use super::SecretsStore;
use chrono::Utc;
use log::error;
use std::sync::{Arc, Weak};
use std::thread;
use std::time::Duration;

pub struct Autolocker<T> {
  secrets_store: Weak<T>,
}

impl<T> Autolocker<T>
where
  T: SecretsStore + 'static,
{
  pub fn spawn_for(secrets_store: &Arc<T>) {
    let auto_locker = Autolocker {
      secrets_store: Arc::downgrade(secrets_store),
    };

    thread::spawn(move || auto_locker.run());
  }

  fn run(&self) {
    while let Some(secrets_store) = self.secrets_store.upgrade() {
      thread::sleep(Duration::from_secs(1));
      let status = match secrets_store.status() {
        Ok(status) => status,
        Err(error) => {
          error!("Autolocker was unable to query status: {}", error);
          continue;
        }
      };

      if let Some(autolock_at) = status.autolock_at {
        if autolock_at < Utc::now() {
          if let Err(error) = secrets_store.lock() {
              error!("Autolocker was unable to lock store: {}", error);
          }
        }
      }
    }
  }
}
