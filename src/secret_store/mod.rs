use crate::api::{
  Identity, PasswordEstimate, PasswordStrength, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status,
};

mod cipher;
mod error;

pub use self::error::{SecretStoreError, SecretStoreResult};

pub trait Secrets {
  fn status() -> SecretStoreResult<Status>;

  fn lock() -> SecretStoreResult<()>;
  fn unlock(name: &String, email: &String, passphrase: &String) -> SecretStoreResult<()>;

  fn identities() -> SecretStoreResult<Vec<Identity>>;

  fn list(filter: &SecretListFilter) -> SecretStoreResult<SecretList>;

  fn add(id: &String, secret_type: SecretType, secret_version: &SecretVersion) -> SecretStoreResult<()>;
  fn get(id: &String) -> SecretStoreResult<Secret>;

  fn estimate_strength(estimate: &PasswordEstimate) -> SecretStoreResult<PasswordStrength>;
}
