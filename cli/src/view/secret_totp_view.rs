use cursive::traits::{Nameable, Resizable};
use cursive::view::{Finder, ViewWrapper};
use cursive::views::{Button, LinearLayout, ProgressBar, TextView};
use cursive::Cursive;
use std::time::{SystemTime, UNIX_EPOCH};
use t_rust_less_lib::otp::{OTPAuthUrl, OTPType};
use zeroize::Zeroize;

pub struct SecretTOTPView {
  base_view: LinearLayout,
  otp_url: String,
  token_display_id: String,
  token_valid_id: String,
  maybe_valid_until: Option<u64>,
}

impl SecretTOTPView {
  pub fn new<F>(property: &str, otp_url: &str, on_copy: F) -> Self
  where
    F: Fn(&mut Cursive) + 'static,
  {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let (token, maybe_valid_until, maybe_period) = match OTPAuthUrl::parse(otp_url) {
      Ok(otpauth) => {
        let (token, valid_until) = otpauth.generate(now);
        let period = match otpauth.otp_type {
          OTPType::Totp { period } => Some(period),
          _ => None,
        };
        (token, Some(valid_until), period)
      }
      _ => ("".to_string(), None, None),
    };
    let token_display_id = format!("token_display_{}", property);
    let token_valid_id = format!("token_valid_{}", property);
    let mut token_display =
      LinearLayout::vertical().child(TextView::new(token).with_name(token_display_id.clone()).full_width());

    if let Some(period) = maybe_period {
      token_display = token_display.child(
        ProgressBar::new()
          .max(period as usize)
          .with_name(token_valid_id.clone())
          .full_width(),
      )
    }

    SecretTOTPView {
      base_view: LinearLayout::horizontal()
        .child(TextView::new(format!("{:10}: ", property)))
        .child(token_display.full_width())
        .child(Button::new("Copy", on_copy)),
      otp_url: otp_url.to_string(),
      token_display_id,
      token_valid_id,
      maybe_valid_until,
    }
  }
}

impl Drop for SecretTOTPView {
  fn drop(&mut self) {
    self.otp_url.zeroize();
  }
}

impl ViewWrapper for SecretTOTPView {
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
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    if let Some(valid_until) = self.maybe_valid_until {
      if valid_until <= now {
        if let Ok(otpauth) = OTPAuthUrl::parse(&self.otp_url) {
          let (token, valid_until) = otpauth.generate(now);
          let mut token_display = self.base_view.find_name::<TextView>(&self.token_display_id).unwrap();
          token_display.set_content(token);
          self.maybe_valid_until = Some(valid_until);
        }
      }
      if let Some(mut token_valid) = self.base_view.find_name::<ProgressBar>(&self.token_valid_id) {
        token_valid.set_value((valid_until - now) as usize);
      }
    }

    Some(f(&mut self.base_view))
  }
}
