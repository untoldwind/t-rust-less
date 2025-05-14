use cursive::traits::Resizable;
use cursive::view::ViewWrapper;
use cursive::views::{Button, LinearLayout, TextArea, TextView};
use cursive::Cursive;

pub struct SecretNodeView {
  base_view: LinearLayout,
}

impl SecretNodeView {
  pub fn new<F>(property: &str, value: &str, on_copy: F) -> Self
  where
    F: Fn(&mut Cursive) + Sync + Send + 'static,
  {
    SecretNodeView {
      base_view: LinearLayout::horizontal()
        .child(TextView::new(format!("{:10}: ", property)))
        .child(TextArea::new().disabled().content(value).full_width().min_height(3))
        .child(Button::new("Copy", on_copy)),
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
