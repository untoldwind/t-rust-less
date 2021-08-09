use crate::clipboard::ClipboardError;
use crate::secrets_store::SecretStoreError;
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroize;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub enum ServiceError {
  SecretsStore(SecretStoreError),
  IO(String),
  Mutex(String),
  StoreNotFound(String),
  ClipboardClosed,
  NotAvailable,
}

impl std::error::Error for ServiceError {}

impl fmt::Display for ServiceError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      ServiceError::SecretsStore(error) => write!(f, "SecretsStoreError: {}", error)?,
      ServiceError::IO(error) => write!(f, "IO: {}", error)?,
      ServiceError::Mutex(error) => write!(f, "Mutex: {}", error)?,
      ServiceError::StoreNotFound(name) => write!(f, "Store with name {} not found", name)?,
      ServiceError::ClipboardClosed => write!(f, "Clipboard closed")?,
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
error_convert_from!(futures::task::SpawnError, ServiceError, IO(display));
error_convert_from!(serde_json::Error, ServiceError, IO(display));
error_convert_from!(rmp_serde::encode::Error, ServiceError, IO(display));
error_convert_from!(rmp_serde::decode::Error, ServiceError, IO(display));

impl<T> From<std::sync::PoisonError<T>> for ServiceError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    ServiceError::Mutex(format!("{}", error))
  }
}

impl From<capnp::Error> for ServiceError {
  fn from(error: capnp::Error) -> Self {
    match error.kind {
      capnp::ErrorKind::Failed => {
        match serde_json::from_str::<ServiceError>(error.description.trim_start_matches("remote exception: ")) {
          Ok(service_error) => service_error,
          _ => ServiceError::IO(format!("{}", error)),
        }
      }
      _ => ServiceError::IO(format!("{}", error)),
    }
  }
}

impl From<ServiceError> for capnp::Error {
  fn from(error: ServiceError) -> capnp::Error {
    match serde_json::to_string(&error) {
      Ok(json) => capnp::Error {
        kind: capnp::ErrorKind::Failed,
        description: json,
      },
      _ => capnp::Error {
        kind: capnp::ErrorKind::Failed,
        description: format!("{}", error),
      },
    }
  }
}
