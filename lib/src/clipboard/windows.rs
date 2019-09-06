use super::{ClipboardResult, SelectionProvider};

pub struct Clipboard {}

impl Clipboard {
  pub fn new<T>(selection_provider: T) -> ClipboardResult<Clipboard>
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
}
