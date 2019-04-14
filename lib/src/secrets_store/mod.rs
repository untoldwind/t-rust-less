use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status};

mod cipher;
mod error;
mod multi_lane;
mod padding;

#[cfg(test)]
mod tests;

pub use self::error::{SecretStoreError, SecretStoreResult};
use crate::block_store::open_block_store;
use crate::memguard::SecretBytes;

pub trait SecretsStore {
  fn status(&self) -> SecretStoreResult<Status>;

  fn lock(&mut self) -> SecretStoreResult<()>;
  fn unlock(&mut self, identity: &Identity, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn identities(&self) -> SecretStoreResult<Vec<Identity>>;
  fn add_identity(&mut self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()>;
  fn change_passphrase(&mut self, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList>;

  fn add(&mut self, id: &str, secret_type: SecretType, secret_version: SecretVersion) -> SecretStoreResult<()>;
  fn get(&self, id: &str) -> SecretStoreResult<Secret>;
}

pub fn open_secrets_store(url: &str) -> SecretStoreResult<Box<SecretsStore>> {
  let (scheme, block_store_url) = match url.find('+') {
    Some(idx) => (&url[..idx], &url[idx + 1..]),
    _ => return Err(SecretStoreError::InvalidStoreUrl(url.to_string())),
  };

  let block_store = open_block_store(block_store_url)?;

  match scheme {
    "multilane" => Ok(Box::new(multi_lane::MultiLaneSecretsStore::new(block_store))),
    _ => Err(SecretStoreError::InvalidStoreUrl(url.to_string())),
  }
}
