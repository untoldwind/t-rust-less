use cursive::traits::Resizable;
use cursive::view::ViewWrapper;
use cursive::views::{Button, LinearLayout, TextView};
use cursive::Cursive;

pub struct SecretCopyView {
  base_view: LinearLayout,
}

impl SecretCopyView {
  pub fn new<F>(property: &str, value: &str, on_copy: F) -> Self
  where
    F: Fn(&mut Cursive) + Sync + Send + 'static,
  {
    SecretCopyView {
      base_view: LinearLayout::horizontal()
        .child(TextView::new(format!("{property:10}: ")))
        .child(TextView::new(value).full_width())
        .child(Button::new("Copy", on_copy)),
    }
  }
}

impl ViewWrapper for SecretCopyView {
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
