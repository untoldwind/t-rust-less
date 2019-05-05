use cursive::views::{LinearLayout, TextView, TextArea};
use cursive::view::ViewWrapper;

pub struct SecretNodeView {
  base_view: LinearLayout,
}

impl SecretNodeView {
  pub fn new(property: &str, value: &str) -> Self {
    SecretNodeView {
      base_view: LinearLayout::horizontal()
          .child(TextView::new(format!("{}:", property)))
          .child(TextArea::new().disabled().content(value)),
    }
  }
}

impl ViewWrapper for SecretNodeView {
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
