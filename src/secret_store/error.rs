use std::fmt;

pub enum SecretStoreError {
  InvalidStoreUrl,
}

impl fmt::Display for SecretStoreError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    unimplemented!()
  }
}

pub type SecretStoreResult<T> = Result<T, SecretStoreError>;
