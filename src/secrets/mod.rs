use api::{Identity, SecretList, SecretListFilter, SecretType, SecretVersion, Status};
use api::Secret;
use self::error::Result;
use api::PasswordEstimate;
use api::PasswordStrength;
use std::fmt::Debug;

mod error;
mod pgp;

pub trait Secrets: Debug {
    fn status() -> Result<Status>;

    fn lock() -> Result<()>;
    fn unlock(name: &String, email: &String, passphrase: &String) -> Result<()>;

    fn identities() -> Result<Vec<Identity>>;

    fn list(filter: &SecretListFilter) -> Result<SecretList>;

    fn add(id: &String, secret_type: SecretType, secret_version: &SecretVersion) -> Result<()>;
    fn get(id: &String) -> Result<Secret>;

    fn estimate_strength(estimate: &PasswordEstimate) -> Result<PasswordStrength>;
}
