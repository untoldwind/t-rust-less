use serde::{Deserialize, Serialize};
use std::convert::From;
use std::fmt;
use zeroize::Zeroize;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize, Clone)]
#[zeroize(drop)]
pub enum StoreError {
  InvalidBlock(String),
  InvalidStoreUrl(String),
  IO(String),
  Mutex(String),
  Conflict(String),
  StoreNotFound(String),
}

impl fmt::Display for StoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      StoreError::InvalidBlock(block_id) => write!(f, "Invalid block: {}", block_id)?,
      StoreError::InvalidStoreUrl(url) => write!(f, "Invalid store url: {}", url)?,
      StoreError::IO(error) => write!(f, "IO: {}", error)?,
      StoreError::Mutex(error) => write!(f, "Internal locking problem: {}", error)?,
      StoreError::Conflict(error) => write!(f, "Conflict: {}", error)?,
      StoreError::StoreNotFound(name) => write!(f, "Store with name {} not found", name)?,
    }
    Ok(())
  }
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
