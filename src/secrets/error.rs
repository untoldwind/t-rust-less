use std::fmt;

pub enum SecretsError {
  InvalidStoreUrl,
}

impl fmt::Display for SecretsError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    unimplemented!()
  }
}

pub type SecretsResult<T> = Result<T, SecretsError>;
