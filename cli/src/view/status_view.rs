use chrono::{DateTime, Utc};
use cursive::view::ViewWrapper;
use cursive::views::TextView;
use std::sync::Arc;
use t_rust_less_lib::api::Status;
use t_rust_less_lib::secrets_store::SecretsStore;

pub struct StatusView {
  secrets_store: Arc<dyn SecretsStore>,
  text_view: TextView,
  last_update: Option<DateTime<Utc>>,
}

impl StatusView {
  pub fn new(secrets_store: Arc<dyn SecretsStore>, status: Status) -> Self {
    StatusView {
      secrets_store,
      text_view: TextView::new(Self::status_text(status)),
      last_update: None,
    }
  }

  fn status_text(status: Status) -> String {
    if status.locked {
      " Locked".to_string()
    } else {
      match status.autolock_at {
        Some(autolock_at) => {
          let timeout = autolock_at - Utc::now();

          format!(" Unlocked {}s", timeout.num_seconds())
        }
        None => " Unlocked".to_string(),
      }
    }
  }
}

impl ViewWrapper for StatusView {
  type V = TextView;

  fn with_view<F, R>(&self, f: F) -> Option<R>
  where
    F: FnOnce(&Self::V) -> R,
  {
    Some(f(&self.text_view))
  }

  fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
  where
    F: FnOnce(&mut Self::V) -> R,
  {
    // Note: Most likely not the place to do this, but I found no better
    let now = Utc::now();
    if self.last_update.is_none() || (now - self.last_update.unwrap()).num_milliseconds() > 500 {
      if let Ok(status) = self.secrets_store.status() {
        self.text_view.set_content(Self::status_text(status));
        self.last_update = Some(now);
      }
    }

    Some(f(&mut self.text_view))
  }
}
