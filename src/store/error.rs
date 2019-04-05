use std::convert::From;
use std::sync::PoisonError;
use std::fmt;

pub enum StoreError {
    InvalidStoreUrl(String),
    IO(String),
    Mutex(String),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unimplemented!()
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