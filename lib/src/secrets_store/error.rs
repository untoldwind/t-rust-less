use crate::block_store::StoreError;
use serde_derive::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretStoreError {
  Locked,
  Forbidden,
  InvalidPassphrase,
  AlreadyUnlocked,
  Conflict,
  KeyDerivation(String),
  Cipher(String),
  IO(String),
  NoRecipient,
  Padding,
  Mutex(String),
  BlockStore(StoreError),
  InvalidStoreUrl(String),
  Json(String),
  InvalidRecipient(String),
  MissingPrivateKey(String),
  NotFound,
}

impl fmt::Display for SecretStoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      SecretStoreError::Locked => write!(f, "Store is locked")?,
      SecretStoreError::Forbidden => write!(f, "Forbidden user")?,
      SecretStoreError::InvalidPassphrase => write!(f, "Invalid passphrase")?,
      SecretStoreError::AlreadyUnlocked => write!(f, "Already unlocked")?,
      SecretStoreError::Conflict => write!(f, "Conflicting ids/id already taken")?,
      SecretStoreError::KeyDerivation(error) => write!(f, "Key derivation error: {}", error)?,
      SecretStoreError::Cipher(error) => write!(f, "Cipher error: {}", error)?,
      SecretStoreError::IO(error) => write!(f, "IO: {}", error)?,
      SecretStoreError::NoRecipient => write!(f, "User is not a recipient of this message")?,
      SecretStoreError::Padding => write!(f, "Invalid data padding")?,
      SecretStoreError::Mutex(error) => write!(f, "Mutex: {}", error)?,
      SecretStoreError::BlockStore(error) => write!(f, "BlockStore: {}", error)?,
      SecretStoreError::InvalidStoreUrl(error) => write!(f, "Invalid store url: {}", error)?,
      SecretStoreError::Json(error) => write!(f, "Json error: {}", error)?,
      SecretStoreError::InvalidRecipient(error) => write!(f, "Invalid recipient: {}", error)?,
      SecretStoreError::MissingPrivateKey(cipher) => write!(f, "Missing private key for cipher: {}", cipher)?,
      SecretStoreError::NotFound => write!(f, "Secret not found")?,
    }
    Ok(())
  }
}

impl std::error::Error for SecretStoreError {}

pub type SecretStoreResult<T> = Result<T, SecretStoreError>;

error_convert_from!(argon2::Error, SecretStoreError, Cipher(display));
error_convert_from!(openssl::error::ErrorStack, SecretStoreError, Cipher(display));
error_convert_from!(std::io::Error, SecretStoreError, IO(display));
error_convert_from!(chacha20_poly1305_aead::DecryptError, SecretStoreError, Cipher(display));
error_convert_from!(capnp::NotInSchema, SecretStoreError, IO(display));
error_convert_from!(serde_json::Error, SecretStoreError, Json(display));
error_convert_from!(StoreError, SecretStoreError, BlockStore(direct));
#[cfg(feature = "rust_crypto")]
error_convert_from!(rsa::errors::Error, SecretStoreError, Cipher(display));
#[cfg(feature = "rust_crypto")]
error_convert_from!(aes_gcm::Error, SecretStoreError, Cipher(display));

impl<T> From<std::sync::PoisonError<T>> for SecretStoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    SecretStoreError::Mutex(format!("{}", error))
  }
}

impl From<capnp::Error> for SecretStoreError {
  fn from(error: capnp::Error) -> Self {
    match error.kind {
      capnp::ErrorKind::Failed => {
        match serde_json::from_str::<SecretStoreError>(&error.description.trim_start_matches("remote exception: ")) {
          Ok(service_error) => service_error,
          _ => SecretStoreError::IO(format!("{}", error)),
        }
      }
      _ => SecretStoreError::IO(format!("{}", error)),
    }
  }
}

impl Into<capnp::Error> for SecretStoreError {
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
