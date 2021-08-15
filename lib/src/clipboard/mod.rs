#[cfg(all(unix, feature = "with_x11"))]
mod debounce;
mod error;
#[cfg(all(unix, not(feature = "with_x11")))]
mod unix_none;
mod unix_x11;
#[cfg(windows)]
mod windows;

pub use self::error::*;
#[cfg(all(unix, not(feature = "with_x11")))]
pub use self::unix_none::Clipboard;
#[cfg(all(unix, feature = "with_x11"))]
pub use self::unix_x11::Clipboard;
#[cfg(windows)]
pub use self::windows::Clipboard;

pub trait SelectionProvider: Send + Sync {
  fn current_selection_name(&self) -> Option<String>;

  fn get_selection(&mut self) -> Option<String>;
}
