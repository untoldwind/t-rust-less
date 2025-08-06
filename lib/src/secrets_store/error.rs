use crate::block_store::StoreError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zeroize::Zeroize;

#[derive(Debug, Error, PartialEq, Eq, Serialize, Deserialize, Zeroize, Clone)]
#[cfg_attr(feature = "with_specta", derive(specta::Type))]
#[zeroize(drop)]
pub enum SecretStoreError {
  #[error("Store is locked")]
  Locked,
  #[error("Forbidden user")]
  Forbidden,
  #[error("Invalid passphrase")]
  InvalidPassphrase,
  #[error("Already unlocked")]
  AlreadyUnlocked,
  #[error("Conflicting ids/id already taken")]
  Conflict,
  #[error("Key derivation error: {0}")]
  KeyDerivation(String),
  #[error("Cipher error: {0}")]
  Cipher(String),
  #[error("IO: {0}")]
  IO(String),
  #[error("User is not a recipient of this message")]
  NoRecipient,
  #[error("Invalid data padding")]
  Padding,
  #[error("Mutex: {0}")]
  Mutex(String),
  #[error("BlockStore: {0}")]
  BlockStore(StoreError),
  #[error("Invalid store url: {0}")]
  InvalidStoreUrl(String),
  #[error("Json error: {0}")]
  Json(String),
  #[error("Invalid recipient: {0}")]
  InvalidRecipient(String),
  #[error("Missing private key for cipher: {0}")]
  MissingPrivateKey(String),
  #[error("Secret not found")]
  NotFound,
}

pub type SecretStoreResult<T> = Result<T, SecretStoreError>;

error_convert_from!(argon2::Error, SecretStoreError, Cipher(display));
#[cfg(feature = "openssl")]
error_convert_from!(openssl::error::ErrorStack, SecretStoreError, Cipher(display));
error_convert_from!(std::io::Error, SecretStoreError, IO(display));
error_convert_from!(std::str::Utf8Error, SecretStoreError, IO(display));
error_convert_from!(chacha20_poly1305_aead::DecryptError, SecretStoreError, Cipher(display));
error_convert_from!(capnp::NotInSchema, SecretStoreError, IO(display));
error_convert_from!(serde_json::Error, SecretStoreError, Json(display));
error_convert_from!(StoreError, SecretStoreError, BlockStore(direct));
#[cfg(feature = "rust_crypto")]
error_convert_from!(rsa::errors::Error, SecretStoreError, Cipher(display));
#[cfg(feature = "rust_crypto")]
error_convert_from!(aes_gcm::Error, SecretStoreError, Cipher(display));
#[cfg(feature = "rust_crypto")]
error_convert_from!(rsa::pkcs1::Error, SecretStoreError, Cipher(display));
#[cfg(feature = "rust_crypto")]
error_convert_from!(rsa::pkcs8::Error, SecretStoreError, Cipher(display));
#[cfg(feature = "rust_crypto")]
error_convert_from!(rsa::pkcs1::der::Error, SecretStoreError, Cipher(display));
error_convert_from!(rsa::pkcs8::spki::Error, SecretStoreError, Cipher(display));
error_convert_from!(rmp_serde::encode::Error, SecretStoreError, IO(display));
error_convert_from!(rmp_serde::decode::Error, SecretStoreError, IO(display));

impl<T> From<std::sync::PoisonError<T>> for SecretStoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    SecretStoreError::Mutex(format!("{error}"))
  }
}

impl From<capnp::Error> for SecretStoreError {
  fn from(error: capnp::Error) -> Self {
    match error.kind {
      capnp::ErrorKind::Failed => {
        match serde_json::from_str::<SecretStoreError>(error.extra.trim_start_matches("remote exception: ")) {
          Ok(service_error) => service_error,
          _ => SecretStoreError::IO(format!("{error}")),
        }
      }
      _ => SecretStoreError::IO(format!("{error}")),
    }
  }
}

impl From<SecretStoreError> for capnp::Error {
  fn from(error: SecretStoreError) -> capnp::Error {
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
