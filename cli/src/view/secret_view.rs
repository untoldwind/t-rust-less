use crate::view::{SecretSimpleView, SecretTypeView};
use cursive::view::ViewWrapper;
use cursive::views::LinearLayout;
use std::sync::Arc;
use t_rust_less_lib::secrets_store::SecretsStore;

pub struct SecretView {
  secrets_store: Arc<SecretsStore>,
  base_view: Option<LinearLayout>,
}

impl SecretView {
  pub fn new(secrets_store: Arc<SecretsStore>, maybe_secret_id: Option<String>) -> Self {
    let mut view = SecretView {
      secrets_store,
      base_view: None,
    };

    if let Some(secret_id) = maybe_secret_id {
      view.show_secret(&secret_id)
    }
    view
  }

  pub fn clear(&mut self) {
    self.base_view = None;
  }

  pub fn show_secret(&mut self, secret_id: &str) {
    match self.secrets_store.get(secret_id) {
      Ok(secret) => {
        self.base_view = Some(
          LinearLayout::vertical()
            .child(SecretSimpleView::new("Name", &secret.current.name))
            .child(SecretTypeView::new(secret.current.secret_type)),
        )
      }
      _ => self.base_view = None,
    }
  }
}

impl ViewWrapper for SecretView {
  type V = LinearLayout;

  fn with_view<F, R>(&self, f: F) -> Option<R>
  where
    F: FnOnce(&Self::V) -> R,
  {
    self.base_view.as_ref().map(f)
  }

  fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
  where
    F: FnOnce(&mut Self::V) -> R,
  {
    self.base_view.as_mut().map(f)
  }
}
