use crate::api::{
    Identity, PasswordEstimate, PasswordStrength, Secret, SecretList, SecretListFilter, SecretType, SecretVersion,
    Status,
};

mod error;

pub use self::error::SecretsResult;

pub trait Secrets {
    fn status() -> SecretsResult<Status>;

    fn lock() -> SecretsResult<()>;
    fn unlock(name: &String, email: &String, passphrase: &String) -> SecretsResult<()>;

    fn identities() -> SecretsResult<Vec<Identity>>;

    fn list(filter: &SecretListFilter) -> SecretsResult<SecretList>;

    fn add(id: &String, secret_type: SecretType, secret_version: &SecretVersion) -> SecretsResult<()>;
    fn get(id: &String) -> SecretsResult<Secret>;

    fn estimate_strength(estimate: &PasswordEstimate) -> SecretsResult<PasswordStrength>;
}
