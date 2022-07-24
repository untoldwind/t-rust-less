use crate::{api::ClipboardProviding, clipboard::SelectionProvider};
use std::time::SystemTime;
use zeroize::{Zeroize, Zeroizing};

pub struct SelectionProviderHolder {
  provider: Box<dyn SelectionProvider>,
  initialized: SystemTime,
  last_moved: Option<SystemTime>,
  last_content: Option<Zeroizing<String>>,
}

impl SelectionProviderHolder {
  pub fn new<T>(provider: T) -> Self
  where
    T: SelectionProvider + 'static,
  {
    SelectionProviderHolder {
      provider: Box::new(provider),
      initialized: SystemTime::now(),
      last_moved: None,
      last_content: None,
    }
  }

  pub fn get_value(&mut self) -> Option<Zeroizing<String>> {
    let now = SystemTime::now();

    if now
      .duration_since(self.initialized)
      .ok()
      .filter(|elapsed| elapsed.as_millis() < 200)
      .is_some()
    {
      return Some("".to_string().into());
    }

    if self
      .last_moved
      .and_then(|last| now.duration_since(last).ok())
      .filter(|elapsed| elapsed.as_millis() < 200)
      .is_none()
    {
      self.last_content = self.provider.get_selection_value();
      self.last_moved.replace(now);
      self.provider.next_selection();
    }

    self.last_content.clone()
  }

  pub fn current_selection(&self) -> Option<ClipboardProviding> {
    self.provider.current_selection()
  }
}

impl Drop for SelectionProviderHolder {
  fn drop(&mut self) {
    self.last_content.zeroize()
  }
}
