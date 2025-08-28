use crate::secrets_store::SecretStoreError;
use crate::{block_store::StoreError, clipboard::ClipboardError};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Debug, Error, PartialEq, Eq, Serialize, Deserialize, Zeroize, Clone)]
#[cfg_attr(feature = "with_specta", derive(specta::Type))]
#[zeroize(drop)]
pub enum ServiceError {
  #[error("SecretsStoreError: {0}")]
  SecretsStore(SecretStoreError),
  #[error("StoreError: {0}")]
  StoreError(StoreError),
  #[error("IO: {0}")]
  IO(String),
  #[error("Mutex: {0}")]
  Mutex(String),
  #[error("Store with name {0} not found")]
  StoreNotFound(String),
  #[error("Clipboard closed")]
  ClipboardClosed,
  #[error("Functionality not available (on your platform)")]
  NotAvailable,
}

pub type ServiceResult<T> = Result<T, ServiceError>;

error_convert_from!(std::io::Error, ServiceError, IO(display));
error_convert_from!(toml::de::Error, ServiceError, IO(display));
error_convert_from!(SecretStoreError, ServiceError, SecretsStore(direct));
error_convert_from!(StoreError, ServiceError, StoreError(direct));
error_convert_from!(futures::task::SpawnError, ServiceError, IO(display));
error_convert_from!(serde_json::Error, ServiceError, IO(display));
error_convert_from!(rmp_serde::encode::Error, ServiceError, IO(display));
error_convert_from!(rmp_serde::decode::Error, ServiceError, IO(display));

impl From<ClipboardError> for ServiceError {
  fn from(value: ClipboardError) -> Self {
    match value {
      ClipboardError::Unavailable => ServiceError::NotAvailable,
      ClipboardError::Mutex(err) => ServiceError::Mutex(err),
      ClipboardError::Other(err) => ServiceError::IO(err),
    }
  }
}

impl<T> From<std::sync::PoisonError<T>> for ServiceError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    ServiceError::Mutex(format!("{error}"))
  }
}

impl From<capnp::Error> for ServiceError {
  fn from(error: capnp::Error) -> Self {
    match error.kind {
      capnp::ErrorKind::Failed => {
        match serde_json::from_str::<ServiceError>(error.extra.trim_start_matches("remote exception: ")) {
          Ok(service_error) => service_error,
          _ => ServiceError::IO(format!("{error}")),
        }
      }
      _ => ServiceError::IO(format!("{error}")),
    }
  }
}

impl From<ServiceError> for capnp::Error {
  fn from(error: ServiceError) -> capnp::Error {
    match serde_json::to_string(&error) {
      Ok(json) => capnp::Error {
        kind: capnp::ErrorKind::Failed,
        extra: json,
      },
      _ => capnp::Error {
        kind: capnp::ErrorKind::Failed,
        extra: format!("{error}"),
      },
    }
  }
}
