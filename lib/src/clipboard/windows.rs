use log::error;

use super::{ClipboardResult, SelectionProvider};
use crate::api::{ClipboardProviding, EventData, EventHub};
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
    let clipboard = Clipboard {
      provider: Arc::new(RwLock::new(selection_provider)),
      event_hub,
    };
    clipboard.fill();

    Ok(clipboard)
  }

  pub fn is_open(&self) -> bool {
    self.currently_providing().is_none()
  }

  pub fn currently_providing(&self) -> Option<ClipboardProviding> {
    self.provider.read().ok()?.current_selection()
  }

  pub fn provide_next(&self) {
    match self.provider.write() {
      Ok(mut provider) => provider.next_selection(),
      Err(err) => {
        error!("Unable to lock provider {}", err);
      }
    }
    self.fill();
  }

  pub fn destroy(&self) {
    clipboard_win::set_clipboard_string("").ok();
  }

  pub fn wait(&self) -> ClipboardResult<()> {
    Ok(())
  }

  fn fill(&self) {
    match self.provider.read() {
      Ok(provider) => {
        if let (Some(providing), Some(value)) = (provider.current_selection(), provider.get_selection_value()) {
          match clipboard_win::set_clipboard_string(&value) {
            Ok(_) => self.event_hub.send(EventData::ClipboardProviding(providing)),
            Err(err) => error!("Write to win_clipboard failed {}", err),
          }
        } else {
          self.destroy();
        }
      }
      Err(err) => {
        error!("Unable to lock provider {}", err);
        self.destroy();
      }
    }
  }
}
