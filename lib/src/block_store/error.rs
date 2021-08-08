use serde::{Deserialize, Serialize};
use std::convert::From;
use std::fmt;
use zeroize::Zeroize;

use crate::api::CapnpSerializing;
use crate::api_capnp::store_error;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
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

impl<T> From<std::sync::PoisonError<T>> for StoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    StoreError::Mutex(format!("{}", error))
  }
}

impl CapnpSerializing for StoreError {
  type Owned = store_error::Owned;

  fn from_reader(reader: store_error::Reader) -> capnp::Result<Self> {
    match reader.which()? {
      store_error::Which::InvalidBlock(value) => Ok(StoreError::InvalidBlock(value?.to_string())),
      store_error::Which::InvalidStoreUrl(value) => Ok(StoreError::InvalidStoreUrl(value?.to_string())),
      store_error::Which::Io(value) => Ok(StoreError::IO(value?.to_string())),
      store_error::Which::Mutex(value) => Ok(StoreError::Mutex(value?.to_string())),
      store_error::Which::Conflict(value) => Ok(StoreError::Conflict(value?.to_string())),
      store_error::Which::StoreNotFound(value) => Ok(StoreError::StoreNotFound(value?.to_string())),
    }
  }

  fn to_builder(&self, mut builder: store_error::Builder) -> capnp::Result<()> {
    match self {
      StoreError::InvalidBlock(value) => builder.set_invalid_block(value),
      StoreError::InvalidStoreUrl(value) => builder.set_invalid_store_url(value),
      StoreError::IO(value) => builder.set_io(value),
      StoreError::Mutex(value) => builder.set_mutex(value),
      StoreError::Conflict(value) => builder.set_conflict(value),
      StoreError::StoreNotFound(value) => builder.set_store_not_found(value),
    }
    Ok(())
  }
}
