mod error;
#[cfg(unix)]
mod unix;

pub use self::error::*;
#[cfg(unix)]
pub use self::unix::Clipboard;

pub trait SelectionProvider: Send + Sync {
  fn get_selection(&mut self) -> Option<String>;
}
