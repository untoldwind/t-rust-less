use std::fmt;

#[derive(Debug)]
pub enum SecretStoreError {
  KeyDerivation(String),
  Cipher(String),
}

impl fmt::Display for SecretStoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      SecretStoreError::KeyDerivation(error) => write!(f, "Key derivation error: {}", error)?,
      SecretStoreError::Cipher(error) => write!(f, "Cipher error: {}", error)?,
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
    SecretStoreError::KeyDerivation(format!("{}", error))
  }
}
