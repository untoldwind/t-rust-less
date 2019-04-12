#[derive(Debug)]
pub struct ClipboardError(pub String);

pub type ClipboardResult<T> = Result<T, ClipboardError>;

#[cfg(unix)]
impl From<xcb::base::ConnError> for ClipboardError {
  fn from(error: xcb::base::ConnError) -> Self {
    ClipboardError(format!("{}", error))
  }
}

#[cfg(unix)]
impl<T> From<xcb::base::Error<T>> for ClipboardError {
  fn from(error: xcb::base::Error<T>) -> Self {
    ClipboardError(format!("{}", error))
  }
}
