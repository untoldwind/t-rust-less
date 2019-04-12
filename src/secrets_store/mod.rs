use crate::api::{
  Identity, PasswordEstimate, PasswordStrength, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status,
};

mod cipher;
mod dual_lane;
mod error;
mod padding;

pub use self::error::{SecretStoreError, SecretStoreResult};
use crate::memguard::SecretBytes;

pub trait SecretsStore {
  fn status() -> SecretStoreResult<Status>;

  fn lock() -> SecretStoreResult<()>;
  fn unlock(identity: &Identity, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn identities() -> SecretStoreResult<Vec<Identity>>;
  fn add_identity(identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()>;

  fn list(filter: &SecretListFilter) -> SecretStoreResult<SecretList>;

  fn add(id: &str, secret_type: SecretType, secret_version: SecretVersion) -> SecretStoreResult<()>;
  fn get(id: &str) -> SecretStoreResult<Secret>;
}
