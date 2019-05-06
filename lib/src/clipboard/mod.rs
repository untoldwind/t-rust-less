mod debounce;
mod error;
mod unix_x11;
mod unix_xcb;
#[cfg(windows)]
mod windows;

pub use self::error::*;
#[cfg(all(unix, feature = "with_x11"))]
pub use self::unix_x11::Clipboard;
#[cfg(all(unix, feature = "with_xcb"))]
pub use self::unix_xcb::Clipboard;
#[cfg(windows)]
pub use self::windows::Clipboard;

pub trait SelectionProvider: Send + Sync {
  type Content: AsRef<[u8]> + Send + Sync + Clone;

  fn get_selection(&mut self) -> Option<Self::Content>;
}
