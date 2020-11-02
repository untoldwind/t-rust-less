use crate::api::{EventHub, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status};
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

pub trait SecretsStore: std::fmt::Debug {
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

pub fn open_secrets_store(
  name: &str,
  url: &str,
  node_id: &str,
  autolock_timeout: Duration,
  event_hub: Arc<dyn EventHub>,
) -> SecretStoreResult<Arc<dyn SecretsStore>> {
  let (scheme, block_store_url) = match url.find('+') {
    Some(idx) => (&url[..idx], &url[idx + 1..]),
    _ => return Err(SecretStoreError::InvalidStoreUrl(url.to_string())),
  };

  let block_store = open_block_store(block_store_url, node_id)?;

  let secrets_store = match scheme {
    "multilane" => Arc::new(multi_lane::MultiLaneSecretsStore::new(
      name,
      block_store,
      autolock_timeout,
      event_hub,
    )),
    _ => return Err(SecretStoreError::InvalidStoreUrl(url.to_string())),
  };

  Ok(secrets_store)
}
