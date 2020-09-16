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
    unimplemented!()
  }

  pub fn is_open(&self) -> bool {
    unimplemented!()
  }

  pub fn currently_providing(&self) -> Option<String> {
    unimplemented!()
  }

  pub fn provide_next(&self) {
    unimplemented!()
  }

  pub fn destroy(&self) {
    unimplemented!()
  }
}
