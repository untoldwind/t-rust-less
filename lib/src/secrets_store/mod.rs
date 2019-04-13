use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status};

mod cipher;
mod error;
mod multi_lane;
mod padding;

pub use self::error::{SecretStoreError, SecretStoreResult};
use crate::memguard::SecretBytes;

pub trait SecretsStore {
  fn status(&self) -> SecretStoreResult<Status>;

  fn lock(&mut self) -> SecretStoreResult<()>;
  fn unlock(&mut self, identity: &Identity, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn identities(&self) -> SecretStoreResult<Vec<Identity>>;
  fn add_identity(&mut self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList>;

  fn add(&mut self, id: &str, secret_type: SecretType, secret_version: SecretVersion) -> SecretStoreResult<()>;
  fn get(&self, id: &str) -> SecretStoreResult<Secret>;
}
