use crate::{api::CapnpSerializing, api_capnp::secret_store_error, block_store::StoreError};
use serde::{Deserialize, Serialize};
use std::fmt;
use zeroize::Zeroize;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
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
#[cfg(feature = "openssl")]
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
        match serde_json::from_str::<SecretStoreError>(error.description.trim_start_matches("remote exception: ")) {
          Ok(service_error) => service_error,
          _ => SecretStoreError::IO(format!("{}", error)),
        }
      }
      _ => SecretStoreError::IO(format!("{}", error)),
    }
  }
}

impl From<SecretStoreError> for capnp::Error {
  fn from(error: SecretStoreError) -> capnp::Error {
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

impl CapnpSerializing for SecretStoreError {
  type Owned = secret_store_error::Owned;

  fn from_reader(reader: secret_store_error::Reader) -> capnp::Result<Self> {
    match reader.which()? {
      secret_store_error::Locked(_) => Ok(SecretStoreError::Locked),
      secret_store_error::Forbidden(_) => Ok(SecretStoreError::Forbidden),
      secret_store_error::InvalidPassphrase(_) => Ok(SecretStoreError::InvalidPassphrase),
      secret_store_error::AlreadyUnlocked(_) => Ok(SecretStoreError::AlreadyUnlocked),
      secret_store_error::Conflict(_) => Ok(SecretStoreError::Conflict),
      secret_store_error::KeyDerivation(value) => Ok(SecretStoreError::KeyDerivation(value?.to_string())),
      secret_store_error::Cipher(value) => Ok(SecretStoreError::Cipher(value?.to_string())),
      secret_store_error::Io(value) => Ok(SecretStoreError::IO(value?.to_string())),
      secret_store_error::NoRecipient(_) => Ok(SecretStoreError::NoRecipient),
      secret_store_error::Padding(_) => Ok(SecretStoreError::Padding),
      secret_store_error::Mutex(value) => Ok(SecretStoreError::Mutex(value?.to_string())),
      secret_store_error::BlockStore(store_error) => {
        Ok(SecretStoreError::BlockStore(StoreError::from_reader(store_error?)?))
      }
      secret_store_error::InvalidStoreUrl(value) => Ok(SecretStoreError::InvalidStoreUrl(value?.to_string())),
      secret_store_error::Json(value) => Ok(SecretStoreError::Json(value?.to_string())),
      secret_store_error::InvalidRecipient(value) => Ok(SecretStoreError::InvalidRecipient(value?.to_string())),
      secret_store_error::MissingPrivateKey(value) => Ok(SecretStoreError::MissingPrivateKey(value?.to_string())),
      secret_store_error::NotFound(_) => Ok(SecretStoreError::NotFound),
    }
  }

  fn to_builder(&self, mut builder: secret_store_error::Builder) -> capnp::Result<()> {
    match self {
      SecretStoreError::Locked => builder.set_locked(()),
      SecretStoreError::Forbidden => builder.set_forbidden(()),
      SecretStoreError::InvalidPassphrase => builder.set_invalid_passphrase(()),
      SecretStoreError::AlreadyUnlocked => builder.set_already_unlocked(()),
      SecretStoreError::Conflict => builder.set_conflict(()),
      SecretStoreError::KeyDerivation(value) => builder.set_key_derivation(value),
      SecretStoreError::Cipher(value) => builder.set_cipher(value),
      SecretStoreError::IO(value) => builder.set_io(value),
      SecretStoreError::NoRecipient => builder.set_no_recipient(()),
      SecretStoreError::Padding => builder.set_padding(()),
      SecretStoreError::Mutex(value) => builder.set_mutex(value),
      SecretStoreError::BlockStore(value) => value.to_builder(builder.init_block_store())?,
      SecretStoreError::InvalidStoreUrl(value) => builder.set_invalid_store_url(value),
      SecretStoreError::Json(value) => builder.set_json(value),
      SecretStoreError::InvalidRecipient(value) => builder.set_invalid_recipient(value),
      SecretStoreError::MissingPrivateKey(value) => builder.set_missing_private_key(value),
      SecretStoreError::NotFound => builder.set_not_found(()),
    }
    Ok(())
  }
}
