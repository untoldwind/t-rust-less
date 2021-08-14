use super::{ClipboardResult, SelectionProvider};
use crate::api::{ClipboardProviding, EventHub};
use std::sync::Arc;

pub struct Clipboard {}

impl Clipboard {
  pub fn new<T>(
    _display_name: &str,
    _selection_provider: T,
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

  pub fn currently_providing(&self) -> Option<ClipboardProviding> {
    unimplemented!()
  }

  pub fn provide_next(&self) {
    unimplemented!()
  }

  pub fn destroy(&self) {
    unimplemented!()
  }

  pub fn wait(&self) -> ClipboardResult<()> {
    unimplemented!()
  }
}
