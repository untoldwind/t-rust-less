#[cfg(all(unix, any(feature = "with_x11", feature = "with_xcb")))]
mod debounce;
mod error;
#[cfg(all(unix, not(any(feature = "with_x11", feature = "with_xcb"))))]
mod unix_none;
mod unix_x11;
mod unix_xcb;
#[cfg(windows)]
mod windows;

pub use self::error::*;
#[cfg(all(unix, not(any(feature = "with_x11", feature = "with_xcb"))))]
pub use self::unix_none::Clipboard;
#[cfg(all(unix, feature = "with_x11"))]
pub use self::unix_x11::Clipboard;
#[cfg(all(unix, feature = "with_xcb", not(feature = "with_x11")))]
pub use self::unix_xcb::Clipboard;
#[cfg(windows)]
pub use self::windows::Clipboard;

pub trait SelectionProvider: Send + Sync {
  fn current_selection_name(&self) -> Option<String>;

  fn get_selection(&mut self) -> Option<String>;
}
