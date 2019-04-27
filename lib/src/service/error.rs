use crate::secrets_store::SecretStoreError;
use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum ServiceError {
  SecretsStore(SecretStoreError),
  IO(String),
  Mutex(String),
  StoreNotFound(String),
  Capnp(String),
}

impl fmt::Display for ServiceError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      ServiceError::SecretsStore(error) => write!(f, "SecretsStoreError: {}", error)?,
      ServiceError::IO(error) => write!(f, "IO: {}", error)?,
      ServiceError::Mutex(error) => write!(f, "Mutex: {}", error)?,
      ServiceError::StoreNotFound(name) => write!(f, "Store with name {} not found", name)?,
      ServiceError::Capnp(error) => write!(f, "Remote protocol error: {}", error)?,
    }
    Ok(())
  }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

error_convert_from!(std::io::Error, ServiceError, IO(display));
error_convert_from!(toml::de::Error, ServiceError, IO(display));
error_convert_from!(SecretStoreError, ServiceError, SecretsStore(direct));
error_convert_from!(capnp::Error, ServiceError, Capnp(display));

impl<T> From<std::sync::PoisonError<T>> for ServiceError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    ServiceError::Mutex(format!("{}", error))
  }
}
