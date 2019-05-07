use std::fmt;

#[derive(Debug)]
pub struct ClipboardError(pub String);

impl fmt::Display for ClipboardError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}

pub type ClipboardResult<T> = Result<T, ClipboardError>;

#[cfg(all(unix, feature = "with_xcb"))]
impl From<xcb::base::ConnError> for ClipboardError {
  fn from(error: xcb::base::ConnError) -> Self {
    ClipboardError(format!("{}", error))
  }
}

#[cfg(all(unix, feature = "with_xcb"))]
impl<T> From<xcb::base::Error<T>> for ClipboardError {
  fn from(error: xcb::base::Error<T>) -> Self {
    ClipboardError(format!("{}", error))
  }
}

impl From<std::ffi::NulError> for ClipboardError {
  fn from(error: std::ffi::NulError) -> Self {
    ClipboardError(format!("{}", error))
  }
}
