use crate::block_store::StoreError;
use std::fmt;

#[derive(Debug)]
pub enum SecretStoreError {
  Locked,
  KeyDerivation(String),
  Cipher(String),
  IO(String),
  NoRecipient,
  Padding,
  Mutex(String),
  BlockStore(StoreError),
  InvalidStoreUrl(String),
}

impl fmt::Display for SecretStoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      SecretStoreError::Locked => write!(f, "Store is locked")?,
      SecretStoreError::KeyDerivation(error) => write!(f, "Key derivation error: {}", error)?,
      SecretStoreError::Cipher(error) => write!(f, "Cipher error: {}", error)?,
      SecretStoreError::IO(error) => write!(f, "IO: {}", error)?,
      SecretStoreError::NoRecipient => write!(f, "User is not a recipient of this message")?,
      SecretStoreError::Padding => write!(f, "Invalid data padding")?,
      SecretStoreError::Mutex(error) => write!(f, "Mutex: {}", error)?,
      SecretStoreError::BlockStore(error) => write!(f, "BlockStore: {}", error)?,
      SecretStoreError::InvalidStoreUrl(error) => write!(f, "Invalid store url: {}", error)?,
    }
    Ok(())
  }
}

pub type SecretStoreResult<T> = Result<T, SecretStoreError>;

error_convert_from!(argon2::Error, SecretStoreError, Cipher(display));
error_convert_from!(openssl::error::ErrorStack, SecretStoreError, Cipher(display));
error_convert_from!(std::io::Error, SecretStoreError, IO(display));
error_convert_from!(chacha20_poly1305_aead::DecryptError, SecretStoreError, Cipher(display));
error_convert_from!(capnp::Error, SecretStoreError, IO(display));
error_convert_from!(capnp::NotInSchema, SecretStoreError, IO(display));
error_convert_from!(StoreError, SecretStoreError, BlockStore(direct));

impl<T> From<std::sync::PoisonError<T>> for SecretStoreError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    SecretStoreError::Mutex(format!("{}", error))
  }
}
