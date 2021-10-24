use super::{ClipboardResult, SelectionProvider};
use crate::api::{ClipboardProviding, EventHub};
use std::sync::{Arc, RwLock};

pub struct Clipboard {
  provider: Arc<RwLock<dyn SelectionProvider>>,
  event_hub: Arc<dyn EventHub>,
}

impl Clipboard {
  pub fn new<T>(_display_name: &str, selection_provider: T, event_hub: Arc<dyn EventHub>) -> ClipboardResult<Clipboard>
  where
    T: SelectionProvider + 'static,
  {
    Ok(Clipboard {
      provider: Arc::new(RwLock::new(selection_provider)),
      event_hub,
    })
  }

  pub fn is_open(&self) -> bool {
    unimplemented!()
  }

  pub fn currently_providing(&self) -> Option<ClipboardProviding> {
    self.provider.read().ok()?.current_selection()
  }

  pub fn provide_next(&self) {
    unimplemented!()
  }

  pub fn destroy(&self) {
    unimplemented!()
  }

  pub fn wait(&self) -> ClipboardResult<()> {
    Ok(())
  }
}
