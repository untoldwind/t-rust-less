use cursive::view::ViewWrapper;
use cursive::views::{LinearLayout, TextView};

pub struct SecretSimpleView {
  base_view: LinearLayout,
}

impl SecretSimpleView {
  pub fn new(property: &str, value: &str) -> Self {
    SecretSimpleView {
      base_view: LinearLayout::horizontal()
        .child(TextView::new(format!("{property:10}: ")))
        .child(TextView::new(value)),
    }
  }
}

impl ViewWrapper for SecretSimpleView {
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
