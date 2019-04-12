mod error;
#[cfg(unix)]
mod unix;

pub use self::error::*;

pub trait SelectionProvider: Send + Sync {
  fn get_selection(&self) -> Option<String>;
}
