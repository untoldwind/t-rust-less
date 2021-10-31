mod error;
#[cfg(all(unix, not(feature = "with_x11")))]
mod unix_none;
mod unix_x11;
#[cfg(windows)]
mod windows;

use zeroize::Zeroizing;

use crate::api::ClipboardProviding;

pub use self::error::*;
#[cfg(all(unix, not(feature = "with_x11")))]
pub use self::unix_none::Clipboard;
#[cfg(all(unix, feature = "with_x11"))]
pub use self::unix_x11::Clipboard;
#[cfg(windows)]
pub use self::windows::Clipboard;

pub trait SelectionProvider: Send + Sync {
  fn current_selection(&self) -> Option<ClipboardProviding>;

  fn get_selection_value(&self) -> Option<Zeroizing<String>>;

  fn next_selection(&mut self);
}
