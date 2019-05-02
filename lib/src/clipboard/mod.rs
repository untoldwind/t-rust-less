mod error;
#[cfg(unix)]
mod unix_x11;
#[cfg(unix)]
mod unix_xcb;
#[cfg(windows)]
mod windows;

pub use self::error::*;
#[cfg(unix)]
pub use self::unix_xcb::Clipboard;
#[cfg(windows)]
pub use self::windows::Clipboard;

pub trait SelectionProvider: Send + Sync {
  fn get_selection(&mut self) -> Option<String>;
}
