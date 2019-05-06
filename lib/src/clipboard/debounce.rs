use super::SelectionProvider;
use std::time::SystemTime;

struct LastContext<C> {
  content: C,
  timestamp: SystemTime,
}

/// Debounce selections.
///
/// Some clients, like a browser that shall not be named (it's from a company start with G),
/// sent up to 4 selection requests to process a single Crtl-V. There seems to be no other way
/// but to debounce these requests by time. I.e. we consider all requests within 200ms to be part
/// of the same paste-action.
///
pub struct SelectionDebounce<T, C> {
  underlying: T,
  last_content: Option<LastContext<C>>,
}

impl<T, C> SelectionDebounce<T, C>
where
  T: SelectionProvider<Content = C>,
  C: AsRef<[u8]> + Send + Sync + Clone,
{
  pub fn new(underlying: T) -> Self {
    SelectionDebounce {
      underlying,
      last_content: None,
    }
  }
}

impl<T, C> SelectionProvider for SelectionDebounce<T, C>
where
  T: SelectionProvider<Content = C>,
  C: AsRef<[u8]> + Send + Sync + Clone,
{
  type Content = C;

  fn get_selection(&mut self) -> Option<Self::Content> {
    let now = SystemTime::now();
    if let Some(last_content) = &self.last_content {
      if let Ok(elapsed) = now.duration_since(last_content.timestamp) {
        if elapsed.as_millis() < 200 {
          return Some(last_content.content.clone());
        }
      }
    }
    match self.underlying.get_selection() {
      Some(content) => {
        self.last_content = Some(LastContext {
          content: content.clone(),
          timestamp: now,
        });
        Some(content)
      }
      _ => {
        self.last_content = None;
        None
      }
    }
  }
}
