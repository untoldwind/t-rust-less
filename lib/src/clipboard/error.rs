use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
#[cfg_attr(feature = "with_specta", derive(specta::Type))]
pub enum ClipboardError {
  #[error("Clipboard not available")]
  Unavailable,
  #[error("Clipboard mutex error: {0}")]
  Mutex(String),
  #[error("Clipboard error: {0}")]
  Other(String),
}

pub type ClipboardResult<T> = Result<T, ClipboardError>;

#[cfg(all(unix, feature = "with_x11"))]
impl From<std::ffi::NulError> for ClipboardError {
  fn from(error: std::ffi::NulError) -> Self {
    ClipboardError::Other(format!("{error}"))
  }
}

#[cfg(all(unix, feature = "with_x11"))]
impl From<std::env::VarError> for ClipboardError {
  fn from(error: std::env::VarError) -> Self {
    ClipboardError::Other(format!("{error}"))
  }
}

#[cfg(all(unix, feature = "with_wayland"))]
impl From<wayland_client::ConnectError> for ClipboardError {
  fn from(error: wayland_client::ConnectError) -> Self {
    match error {
      wayland_client::ConnectError::NoCompositor => ClipboardError::Unavailable,
      wayland_client::ConnectError::NoWaylandLib => ClipboardError::Unavailable,
      err => ClipboardError::Other(format!("{err}")),
    }
  }
}

#[cfg(all(unix, feature = "with_wayland"))]
impl From<wayland_client::globals::GlobalError> for ClipboardError {
  fn from(error: wayland_client::globals::GlobalError) -> Self {
    ClipboardError::Other(format!("{error}"))
  }
}

#[cfg(all(unix, feature = "with_wayland"))]
impl From<wayland_client::globals::BindError> for ClipboardError {
  fn from(error: wayland_client::globals::BindError) -> Self {
    ClipboardError::Other(format!("{error}"))
  }
}

#[cfg(all(unix, feature = "with_wayland"))]
impl From<wayland_client::DispatchError> for ClipboardError {
  fn from(error: wayland_client::DispatchError) -> Self {
    ClipboardError::Other(format!("{error}"))
  }
}

impl<T> From<std::sync::PoisonError<T>> for ClipboardError {
  fn from(error: std::sync::PoisonError<T>) -> Self {
    ClipboardError::Mutex(format!("{error}"))
  }
}
