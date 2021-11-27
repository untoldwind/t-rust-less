mod error;
#[cfg(all(unix, feature = "with_x11", feature = "with_wayland"))]
mod unix_mixed;
#[cfg(all(unix, not(any(feature = "with_x11", feature = "with_wayland"))))]
mod unix_none;
#[cfg(all(unix, feature = "with_wayland"))]
pub mod unix_wayland;
#[cfg(all(unix, feature = "with_x11"))]
mod unix_x11;
#[cfg(windows)]
mod windows;

use zeroize::Zeroizing;

#[cfg(not(windows))]
mod selection_provider_holder;

use std::sync::Arc;

use crate::api::{ClipboardProviding, EventHub};

pub use self::error::*;
#[cfg(all(unix, feature = "with_x11", feature = "with_wayland"))]
pub use self::unix_mixed::Clipboard;
#[cfg(all(unix, not(any(feature = "with_x11", feature = "with_wayland"))))]
pub use self::unix_none::Clipboard;
#[cfg(all(unix, feature = "with_wayland", not(feature = "with_x11")))]
pub use self::unix_wayland::Clipboard;
#[cfg(all(unix, feature = "with_x11", not(feature = "with_wayland")))]
pub use self::unix_x11::Clipboard;
#[cfg(windows)]
pub use self::windows::Clipboard;

pub trait SelectionProvider: Send + Sync {
  fn current_selection(&self) -> Option<ClipboardProviding>;

  fn get_selection_value(&self) -> Option<Zeroizing<String>>;

  fn next_selection(&mut self);
}

pub trait ClipboardCommon: Sized {
  fn new<T>(selection_provider: T, event_hub: Arc<dyn EventHub>) -> ClipboardResult<Self>
  where
    T: SelectionProvider + Clone + 'static;

  fn destroy(&self);

  fn is_open(&self) -> bool;

  fn currently_providing(&self) -> Option<ClipboardProviding>;

  fn provide_next(&self);

  fn wait(&self) -> ClipboardResult<()>;
}
