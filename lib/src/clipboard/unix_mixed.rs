use std::sync::Arc;

use log::info;

use crate::api::EventHub;

use super::{unix_wayland, unix_x11, ClipboardCommon, ClipboardError, ClipboardResult, SelectionProvider};

pub enum Clipboard {
  Wayland(unix_wayland::Clipboard),
  X11(unix_x11::Clipboard),
}

impl ClipboardCommon for Clipboard {
  fn new<T>(selection_provider: T, event_hub: Arc<dyn EventHub>) -> ClipboardResult<Self>
  where
    T: SelectionProvider + Clone + 'static,
  {
    match unix_wayland::Clipboard::new(selection_provider.clone(), event_hub.clone()) {
      Ok(wayland) => Ok(Clipboard::Wayland(wayland)),
      Err(ClipboardError::Unavailable) => {
        info!("Wayland unavailable, fallback to x11");
        unix_x11::Clipboard::new(selection_provider, event_hub).map(Clipboard::X11)
      }
      Err(err) => Err(err),
    }
  }

  fn destroy(&self) {
    match self {
      Clipboard::Wayland(wayland) => wayland.destroy(),
      Clipboard::X11(x11) => x11.destroy(),
    }
  }

  fn is_open(&self) -> bool {
    match self {
      Clipboard::Wayland(wayland) => wayland.is_open(),
      Clipboard::X11(x11) => x11.is_open(),
    }
  }

  fn currently_providing(&self) -> Option<crate::api::ClipboardProviding> {
    match self {
      Clipboard::Wayland(wayland) => wayland.currently_providing(),
      Clipboard::X11(x11) => x11.currently_providing(),
    }
  }

  fn provide_next(&self) {
    match self {
      Clipboard::Wayland(wayland) => wayland.provide_next(),
      Clipboard::X11(x11) => x11.provide_next(),
    }
  }

  fn wait(&self) -> super::ClipboardResult<()> {
    match self {
      Clipboard::Wayland(wayland) => wayland.wait(),
      Clipboard::X11(x11) => x11.wait(),
    }
  }
}
