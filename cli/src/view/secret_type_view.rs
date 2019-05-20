use cursive::view::ViewWrapper;
use cursive::views::{LinearLayout, TextView};
use t_rust_less_lib::api::SecretType;

pub struct SecretTypeView {
  base_view: LinearLayout,
}

impl SecretTypeView {
  pub fn new(secret_type: SecretType) -> Self {
    SecretTypeView {
      base_view: LinearLayout::horizontal()
        .child(TextView::new("Type      : "))
        .child(TextView::new(secret_type.to_string())),
    }
  }
}

impl ViewWrapper for SecretTypeView {
  type V = LinearLayout;

  fn with_view<F, R>(&self, f: F) -> Option<R>
  where
    F: FnOnce(&Self::V) -> R,
  {
    Some(f(&self.base_view))
  }

  fn with_view_mut<F, R>(&mut self, f: F) -> Option<R>
  where
    F: FnOnce(&mut Self::V) -> R,
  {
    Some(f(&mut self.base_view))
  }
}
