use cursive::direction::Direction;
use cursive::event::{Event, EventResult, Key};
use cursive::theme::{ColorStyle, Effect};
use cursive::view::View;
use cursive::{Cursive, Printer, Rect, Vec2, With};
use std::rc::Rc;
use t_rust_less_lib::memguard::SecretBytes;

pub type OnSubmit = dyn Fn(&mut Cursive);

/// A stripped down version of an EditView offering the bare minimum functions to
/// enter a password/passphrase.
/// Contrary to an ordinary EditView backing store is SecretBytes.
pub struct PasswordView {
  content: SecretBytes,

  last_length: usize,

  on_submit: Option<Rc<OnSubmit>>,

  enabled: bool,

  style: ColorStyle,
}

impl PasswordView {
  pub fn new(capacity: usize) -> Self {
    PasswordView {
      content: SecretBytes::with_capacity_for_chars(capacity),
      last_length: 0,
      on_submit: None,
      enabled: true,
      style: ColorStyle::secondary(),
    }
  }

  /// Disables this view.
  ///
  /// A disabled view cannot be selected.
  pub fn disable(&mut self) {
    self.enabled = false;
  }

  /// Disables this view.
  ///
  /// Chainable variant.
  pub fn disabled(self) -> Self {
    self.with(Self::disable)
  }

  /// Re-enables this view.
  pub fn enable(&mut self) {
    self.enabled = true;
  }

  /// Sets the style used for this view.
  ///
  /// When the view is enabled, the style will be reversed.
  ///
  /// Defaults to `ColorStyle::Secondary`.
  pub fn set_style(&mut self, style: ColorStyle) {
    self.style = style;
  }

  /// Sets the style used for this view.
  ///
  /// When the view is enabled, the style will be reversed.
  ///
  /// Chainable variant.
  pub fn style(self, style: ColorStyle) -> Self {
    self.with(|s| s.set_style(style))
  }

  /// Sets a callback to be called when `<Enter>` is pressed.
  ///
  /// `callback` will be given the content of the view.
  ///
  /// This callback can safely trigger itself recursively if needed
  /// (for instance if you call `on_event` on this view from the callback).
  ///
  /// If you need a mutable closure and don't care about the recursive
  /// aspect, see [`set_on_submit_mut`](#method.set_on_submit_mut).
  pub fn set_on_submit<F>(&mut self, callback: F)
  where
    F: Fn(&mut Cursive) + 'static,
  {
    self.on_submit = Some(Rc::new(callback));
  }

  /// Sets a callback to be called when `<Enter>` is pressed.
  ///
  /// Chainable variant.
  pub fn on_submit<F>(self, callback: F) -> Self
  where
    F: Fn(&mut Cursive) + 'static,
  {
    self.with(|v| v.set_on_submit(callback))
  }

  pub fn get_content(&mut self) -> SecretBytes {
    self.content.clone()
  }

  fn append(&mut self, ch: char) {
    if ch.len_utf8() + self.content.len() > self.content.capacity() {
      // Ignore all chars beyond limit
      return;
    }
    self.content.borrow_mut().append_char(ch);
  }

  fn remove(&mut self) {
    if self.content.is_empty() {
      return;
    }
    self.content.borrow_mut().remove_char();
  }

  fn clear(&mut self) {
    self.content.borrow_mut().clear();
  }
}

impl View for PasswordView {
  fn draw(&self, printer: &Printer<'_, '_>) {
    assert_eq!(
      printer.size.x, self.last_length,
      "Was promised {}, received {}",
      self.last_length, printer.size.x
    );

    let width = self.content.borrow().as_str().chars().count();
    printer.with_color(self.style, |printer| {
      let effect = if self.enabled && printer.enabled {
        Effect::Reverse
      } else {
        Effect::Simple
      };
      printer.with_effect(effect, |printer| {
        if width < self.last_length {
          // No problem, everything fits.
          assert!(printer.size.x >= width);
          printer.print_hline((0, 0), width, "*");
          let filler_len = printer.size.x - width;
          printer.print_hline((width, 0), filler_len, "_");
        } else {
          printer.print_hline((0, 0), self.last_length, "*");
        }
      });

      // Now print cursor
      if printer.focused {
        printer.print((width.min(self.last_length), 0), "_");
      }
    });
  }

  fn layout(&mut self, size: Vec2) {
    self.last_length = size.x;
  }

  fn take_focus(&mut self, _: Direction) -> bool {
    self.enabled
  }

  fn on_event(&mut self, event: Event) -> EventResult {
    match event {
      Event::Char(ch) => {
        self.append(ch);
        EventResult::Consumed(None)
      }
      Event::Key(Key::Backspace) => {
        self.remove();
        EventResult::Consumed(None)
      }
      Event::Key(Key::Del) => {
        self.clear();
        EventResult::Consumed(None)
      }
      Event::Key(Key::Enter) if self.on_submit.is_some() => {
        let cb = self.on_submit.clone().unwrap();
        EventResult::with_cb(move |s| {
          cb(s);
        })
      }
      _ => EventResult::Ignored,
    }
  }

  fn important_area(&self, _: Vec2) -> Rect {
    let width = self.content.borrow().as_str().chars().count();
    Rect::from_size((width.min(self.last_length), 0), (1, 1))
  }
}
