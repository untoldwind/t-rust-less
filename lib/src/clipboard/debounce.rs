use super::SelectionProvider;
use std::time::SystemTime;

struct LastContext<C> {
  content: C,
  timestamp: SystemTime,
  initial: bool,
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
  startup_timestamp: SystemTime,
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
      startup_timestamp: SystemTime::now(),
    }
  }
}

impl<T, C> SelectionProvider for SelectionDebounce<T, C>
where
  T: SelectionProvider<Content = C>,
  C: AsRef<[u8]> + Send + Sync + Clone,
{
  type Content = C;

  fn current_selection_name(&self) -> Option<String> {
    self.underlying.current_selection_name()
  }

  fn get_selection(&mut self) -> Option<Self::Content> {
    let now = SystemTime::now();
    if let Some(last_content) = self.last_content.take() {
      if last_content.initial {
        self.last_content.replace(LastContext {
          content: last_content.content.clone(),
          timestamp: now,
          initial: false,
        });
        return Some(last_content.content);
      }
      if let Ok(elapsed) = now.duration_since(last_content.timestamp) {
        if elapsed.as_millis() < 200 {
          let content = last_content.content.clone();
          self.last_content.replace(last_content);
          return Some(content);
        }
      }
    }
    match self.underlying.get_selection() {
      Some(content) => {
        let initial = match now.duration_since(self.startup_timestamp) {
          Ok(elapsed) if elapsed.as_millis() < 200 => true,
          _ => false,
        };
        self.last_content = Some(LastContext {
          content: content.clone(),
          timestamp: now,
          initial
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
