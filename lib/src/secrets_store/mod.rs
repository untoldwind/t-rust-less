use crate::api::{EventHub, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status};
use crate::block_store::sync::SyncBlockStore;
use std::sync::Arc;
use std::time::Duration;

pub mod cipher;
mod error;
pub mod estimate;
mod index;
mod multi_lane;
mod padding;

#[cfg(test)]
mod index_tests;
#[cfg(test)]
mod tests;

pub use self::error::{SecretStoreError, SecretStoreResult};
use crate::block_store::open_block_store;
use crate::memguard::SecretBytes;

pub trait SecretsStore: std::fmt::Debug + Send + Sync {
  fn status(&self) -> SecretStoreResult<Status>;

  fn lock(&self) -> SecretStoreResult<()>;
  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn identities(&self) -> SecretStoreResult<Vec<Identity>>;
  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()>;
  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList>;
  fn update_index(&self) -> SecretStoreResult<()>;

  fn add(&self, secret_version: SecretVersion) -> SecretStoreResult<String>;
  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret>;
  fn get_version(&self, block_id: &str) -> SecretStoreResult<SecretVersion>;
}

#[allow(clippy::type_complexity)]
pub fn open_secrets_store(
  name: &str,
  url: &str,
  maybe_remote_url: Option<&str>,
  node_id: &str,
  autolock_timeout: Duration,
  event_hub: Arc<dyn EventHub>,
) -> SecretStoreResult<(Arc<dyn SecretsStore>, Option<Arc<SyncBlockStore>>)> {
  let (scheme, block_store_url) = match url.find('+') {
    Some(idx) => (&url[..idx], &url[idx + 1..]),
    _ => return Err(SecretStoreError::InvalidStoreUrl(url.to_string())),
  };

  let mut block_store = open_block_store(block_store_url, node_id)?;

  let sync_block_store = match maybe_remote_url {
    Some(remote_url) => {
      let remote = open_block_store(remote_url, node_id)?;

      let sync_block_store = Arc::new(SyncBlockStore::new(block_store, remote));

      block_store = sync_block_store.clone();

      Some(sync_block_store)
    }
    _ => None,
  };

  let secrets_store = match scheme {
    "multilane" => Arc::new(multi_lane::MultiLaneSecretsStore::new(
      name,
      block_store,
      autolock_timeout,
      event_hub,
    )),
    _ => return Err(SecretStoreError::InvalidStoreUrl(url.to_string())),
  };

  Ok((secrets_store, sync_block_store))
}
