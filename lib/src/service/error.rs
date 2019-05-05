use crate::clipboard::ClipboardError;
use crate::secrets_store::SecretStoreError;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceError {
  SecretsStore(SecretStoreError),
  IO(String),
  Mutex(String),
  StoreNotFound(String),
  NotAvailable,
}

impl fmt::Display for ServiceError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      ServiceError::SecretsStore(error) => write!(f, "SecretsStoreError: {}", error)?,
      ServiceError::IO(error) => write!(f, "IO: {}", error)?,
      ServiceError::Mutex(error) => write!(f, "Mutex: {}", error)?,
      ServiceError::StoreNotFound(name) => write!(f, "Store with name {} not found", name)?,
      ServiceError::NotAvailable => write!(f, "Functionality not available (on your platform)")?,
    }
    Ok(())
  }
}

pub type ServiceResult<T> = Result<T, ServiceError>;

error_convert_from!(std::io::Error, ServiceError, IO(display));
error_convert_from!(toml::de::Error, ServiceError, IO(display));
error_convert_from!(SecretStoreError, ServiceError, SecretsStore(direct));
error_convert_from!(ClipboardError, ServiceError, IO(display));

impl<T> From<std::sync::PoisonError<T>> for ServiceError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    ServiceError::Mutex(format!("{}", error))
  }
}

impl From<capnp::Error> for ServiceError {
  fn from(error: capnp::Error) -> Self {
    match error.kind {
      capnp::ErrorKind::Failed => match serde_json::from_str::<ServiceError>(&error.description.trim_start_matches("remote exception: ")) {
        Ok(service_error) => service_error,
        _ => ServiceError::IO(format!("{}", error)),
      },
      _ => ServiceError::IO(format!("{}", error)),
    }
  }
}

impl Into<capnp::Error> for ServiceError {
  fn into(self) -> capnp::Error {
    match serde_json::to_string(&self) {
      Ok(json) => capnp::Error {
        kind: capnp::ErrorKind::Failed,
        description: json,
      },
      _ => capnp::Error {
        kind: capnp::ErrorKind::Failed,
        description: format!("{}", self),
      },
    }
  }
}
