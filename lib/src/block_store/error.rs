use std::convert::From;
use std::fmt;
use std::sync::PoisonError;

#[derive(Debug, PartialEq, Eq)]
pub enum StoreError {
  InvalidBlock(String),
  InvalidStoreUrl(String),
  IO(String),
  Mutex(String),
}

impl fmt::Display for StoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      StoreError::InvalidBlock(block_id) => write!(f, "Invalid block: {}", block_id)?,
      StoreError::InvalidStoreUrl(url) => write!(f, "Invalid store url: {}", url)?,
      StoreError::IO(error) => write!(f, "IO: {}", error)?,
      StoreError::Mutex(error) => write!(f, "Internal locking problem: {}", error)?,
    }
    Ok(())
  }
}

pub type StoreResult<T> = Result<T, StoreError>;

impl From<std::io::Error> for StoreError {
  fn from(error: std::io::Error) -> Self {
    StoreError::IO(format!("{}", error))
  }
}

impl<T> From<std::sync::PoisonError<T>> for StoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    StoreError::Mutex(format!("{}", error))
  }
}

impl From<url::ParseError> for StoreError {
  fn from(error: url::ParseError) -> Self {
    StoreError::InvalidStoreUrl(format!("{}", error))
  }
}
