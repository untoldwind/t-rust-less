use serde::{Deserialize, Serialize};
use std::convert::From;
use std::fmt;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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

impl<T> From<std::sync::PoisonError<T>> for StoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    StoreError::Mutex(format!("{}", error))
  }
}
