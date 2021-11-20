use super::{ClipboardCommon, ClipboardError, ClipboardResult, SelectionProvider};
use crate::api::{ClipboardProviding, EventHub};
use std::sync::Arc;

pub struct Clipboard {}

impl ClipboardCommon for Clipboard {
  fn new<T>(_display_name: &str, _selection_provider: T, _event_hub: Arc<dyn EventHub>) -> ClipboardResult<Self>
  where
    T: SelectionProvider + 'static,
  {
    Err(ClipboardError::Unavailable)
  }

  fn is_open(&self) -> bool {
    false
  }

  fn currently_providing(&self) -> Option<ClipboardProviding> {
    None
  }

  fn provide_next(&self) {}

  fn destroy(&self) {}

  fn wait(&self) -> ClipboardResult<()> {
    Ok(())
  }
}
