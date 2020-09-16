use super::{ClipboardResult, SelectionProvider};
use crate::api::EventHub;
use std::sync::Arc;

pub struct Clipboard {}

impl Clipboard {
  pub fn new<T>(
    _display_name: &str,
    _selection_provider: T,
    _store_name: String,
    _secret_id: String,
    _event_hub: Arc<dyn EventHub>,
  ) -> ClipboardResult<Clipboard>
  where
    T: SelectionProvider + 'static,
  {
    Ok(Clipboard {})
  }

  pub fn is_open(&self) -> bool {
    false
  }

  pub fn currently_providing(&self) -> Option<String> {
    None
  }

  pub fn provide_next(&self) {}

  pub fn destroy(&self) {}
}
