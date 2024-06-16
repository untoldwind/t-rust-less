use serde::{Deserialize, Serialize};
use std::convert::From;
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Debug, Error, PartialEq, Eq, Serialize, Deserialize, Zeroize, Clone)]
#[cfg_attr(feature = "with_specta", derive(specta::Type))]
#[zeroize(drop)]
pub enum StoreError {
  #[error("Invalid block: {0}")]
  InvalidBlock(String),
  #[error("Invalid store url: {0}")]
  InvalidStoreUrl(String),
  #[error("IO: {0}")]
  IO(String),
  #[error("Mutex: {0}")]
  Mutex(String),
  #[error("Conflict: {0}")]
  Conflict(String),
  #[error("Store with name {0} not found")]
  StoreNotFound(String),
}

pub type StoreResult<T> = Result<T, StoreError>;

error_convert_from!(std::io::Error, StoreError, IO(display));
error_convert_from!(url::ParseError, StoreError, InvalidStoreUrl(display));
#[cfg(feature = "sled")]
error_convert_from!(sled::Error, StoreError, IO(display));
#[cfg(feature = "sled")]
error_convert_from!(rmp_serde::encode::Error, StoreError, IO(display));
#[cfg(feature = "sled")]
error_convert_from!(rmp_serde::decode::Error, StoreError, IO(display));
#[cfg(feature = "dropbox")]
error_convert_from!(std::sync::mpsc::RecvError, StoreError, IO(display));
#[cfg(feature = "dropbox")]
error_convert_from!(dropbox_sdk::Error, StoreError, IO(display));
#[cfg(feature = "dropbox")]
error_convert_from!(dropbox_sdk::files::ListFolderError, StoreError, IO(display));
#[cfg(feature = "dropbox")]
error_convert_from!(dropbox_sdk::files::ListFolderContinueError, StoreError, IO(display));
#[cfg(feature = "dropbox")]
error_convert_from!(dropbox_sdk::files::UploadError, StoreError, IO(display));

impl<T> From<std::sync::PoisonError<T>> for StoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    StoreError::Mutex(format!("{}", error))
  }
}

#[cfg(feature = "sled")]
impl From<sled::transaction::TransactionError<StoreError>> for StoreError {
  fn from(err: sled::transaction::TransactionError<StoreError>) -> Self {
    match err {
      sled::transaction::TransactionError::Abort(err) => err,
      sled::transaction::TransactionError::Storage(err) => err.into(),
    }
  }
}
