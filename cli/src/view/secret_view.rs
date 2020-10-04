use crate::error::ExtResult;
use crate::view::{SecretCopyView, SecretNodeView, SecretSimpleView, SecretTOTPView, SecretTypeView};
use cursive::view::ViewWrapper;
use cursive::views::{DummyView, LinearLayout};
use cursive::Cursive;
use std::env;
use std::sync::Arc;
use t_rust_less_lib::api::{Secret, PROPERTY_NOTES, PROPERTY_PASSWORD, PROPERTY_TOTP_URL};
use t_rust_less_lib::secrets_store::SecretsStore;
use t_rust_less_lib::service::TrustlessService;

pub struct SecretView {
  service: Arc<dyn TrustlessService>,
  store_name: String,
  secrets_store: Arc<dyn SecretsStore>,
  base_view: Option<LinearLayout>,
  current_secret: Option<Secret>,
}

impl SecretView {
  pub fn new(
    service: Arc<dyn TrustlessService>,
    store_name: String,
    secrets_store: Arc<dyn SecretsStore>,
    maybe_secret_id: Option<String>,
  ) -> Self {
    let mut view = SecretView {
      service,
      store_name,
      secrets_store,
      base_view: None,
      current_secret: None,
    };

    if let Some(secret_id) = maybe_secret_id {
      view.show_secret(&secret_id)
    }
    view
  }

  pub fn clear(&mut self) {
    self.base_view = None;
  }

  pub fn current_secret(&self) -> Option<Secret> {
    self.current_secret.clone()
  }

  pub fn show_secret(&mut self, secret_id: &str) {
    match self.secrets_store.get(secret_id) {
      Ok(secret) => {
        let mut layout = LinearLayout::vertical()
          .child(SecretSimpleView::new("Name", &secret.current.name))
          .child(SecretTypeView::new(secret.current.secret_type))
          .child(DummyView {});

        for (property, value) in secret.current.properties.iter() {
          match property {
            PROPERTY_PASSWORD => (),
            PROPERTY_NOTES => {
              layout = layout.child(SecretNodeView::new(
                property,
                value,
                self.copy_to_clipboard(secret_id, property),
              ))
            }
            PROPERTY_TOTP_URL => {
              layout = layout.child(SecretTOTPView::new(
                property,
                value,
                self.copy_to_clipboard(secret_id, property),
              ))
            }
            _ => {
              layout = layout.child(SecretCopyView::new(
                property,
                value,
                self.copy_to_clipboard(secret_id, property),
              ))
            }
          }
        }

        self.base_view = Some(layout);
        self.current_secret = Some(secret);
      }
      _ => {
        self.base_view = None;
        self.current_secret = None;
      }
    }
  }

  fn copy_to_clipboard(&self, secret_id: &str, property: &str) -> impl Fn(&mut Cursive) {
    let service = self.service.clone();
    let store_name = self.store_name.clone();
    let owned_secret_id = secret_id.to_string();
    let owned_property = property.to_string();
    move |_: &mut Cursive| {
      service
        .secret_to_clipboard(
          &store_name,
          &owned_secret_id,
          &[&owned_property],
          &env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()),
        )
        .ok_or_exit("Copy to clipboard");
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
