use super::{ClipboardResult, SelectionProvider};
use crate::api::{ClipboardProviding, EventHub};
use std::sync::{Arc, RwLock};

pub struct Clipboard {
  store_name: String,
  block_id: String,
  secret_name: String,
  provider: Arc<RwLock<dyn SelectionProvider>>,
  event_hub: Arc<dyn EventHub>,
}

impl Clipboard {
  pub fn new<T>(
    _display_name: &str,
    _selection_provider: T,
    _event_hub: Arc<dyn EventHub>,
  ) -> ClipboardResult<Clipboard>
  where
    T: SelectionProvider + 'static,
  {
    Ok(Clipboard {
      store_name,
      block_id,
      secret_name,
      provider: Arc::new(RwLock::new(selection_provider)),
      event_hub,
    })
  }

  pub fn is_open(&self) -> bool {
    unimplemented!()
  }

  pub fn currently_providing(&self) -> Option<ClipboardProviding> {
    self
      .provider
      .read()
      .ok()?
      .current_selection_name()
      .map(|property| ClipboardProviding {
        store_name: self.store_name.clone(),
        block_id: self.block_id.clone(),
        secret_name: self.secret_name.clone(),
        property,
      })
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
