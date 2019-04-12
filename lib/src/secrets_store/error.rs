use crate::block_store::StoreError;
use std::fmt;

#[derive(Debug)]
pub enum SecretStoreError {
  KeyDerivation(String),
  Cipher(String),
  IO(String),
  NoRecipient,
  Padding,
  Mutex(String),
  BlockStore(StoreError),
}

impl fmt::Display for SecretStoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      SecretStoreError::KeyDerivation(error) => write!(f, "Key derivation error: {}", error)?,
      SecretStoreError::Cipher(error) => write!(f, "Cipher error: {}", error)?,
      SecretStoreError::IO(error) => write!(f, "IO: {}", error)?,
      SecretStoreError::NoRecipient => write!(f, "User is not a recipient of this message")?,
      SecretStoreError::Padding => write!(f, "Invalid data padding")?,
      SecretStoreError::Mutex(error) => write!(f, "Mutex: {}", error)?,
      SecretStoreError::BlockStore(error) => write!(f, "BlockStore: {}", error)?,
    }
    Ok(())
  }
}

pub type SecretStoreResult<T> = Result<T, SecretStoreError>;

impl From<argon2::Error> for SecretStoreError {
  fn from(error: argon2::Error) -> Self {
    SecretStoreError::KeyDerivation(format!("{}", error))
  }
}

impl From<openssl::error::ErrorStack> for SecretStoreError {
  fn from(error: openssl::error::ErrorStack) -> Self {
    SecretStoreError::Cipher(format!("{}", error))
  }
}

impl From<std::io::Error> for SecretStoreError {
  fn from(error: std::io::Error) -> Self {
    SecretStoreError::IO(format!("{}", error))
  }
}

impl From<chacha20_poly1305_aead::DecryptError> for SecretStoreError {
  fn from(error: chacha20_poly1305_aead::DecryptError) -> Self {
    SecretStoreError::Cipher(format!("{}", error))
  }
}

impl From<capnp::Error> for SecretStoreError {
  fn from(error: capnp::Error) -> Self {
    SecretStoreError::IO(format!("{}", error))
  }
}

impl From<capnp::NotInSchema> for SecretStoreError {
  fn from(error: capnp::NotInSchema) -> Self {
    SecretStoreError::IO(format!("{}", error))
  }
}

impl<T> From<std::sync::PoisonError<T>> for SecretStoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    SecretStoreError::Mutex(format!("{}", error))
  }
}
