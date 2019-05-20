use cursive::traits::{Boxable, Identifiable};
use cursive::view::{Finder, ViewWrapper};
use cursive::views::{Button, LinearLayout, ProgressBar, TextView};
use cursive::Cursive;
use std::time::{SystemTime, UNIX_EPOCH};
use t_rust_less_lib::memguard::weak::{ZeroingString, ZeroingStringExt};
use t_rust_less_lib::otp::{OTPAuthUrl, OTPType};

pub struct SecretTOTPView {
  base_view: LinearLayout,
  otp_url: ZeroingString,
  token_display_id: String,
  token_valid_id: String,
  valid_until: Option<u64>,
  period: Option<u64>,
}

impl SecretTOTPView {
  pub fn new<F>(property: &str, otp_url: &str, on_copy: F) -> Self
  where
    F: Fn(&mut Cursive) -> () + 'static,
  {
    let token_display_id = format!("token_display_{}", property);
    let token_valid_id = format!("token_valid_{}", property);

    SecretTOTPView {
      base_view: LinearLayout::horizontal()
        .child(TextView::new(format!("{:10}: ", property)))
        .child(
          LinearLayout::vertical()
            .child(TextView::new("").with_id(token_display_id.clone()).full_width())
            .child(ProgressBar::new().max(30).with_id(token_valid_id.clone()).full_width())
            .full_width(),
        )
        .child(Button::new("Copy", on_copy)),
      otp_url: otp_url.to_zeroing(),
      token_display_id,
      token_valid_id,
      valid_until: None,
      period: None,
    }
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
    if self.valid_until.is_none() || self.valid_until.unwrap() <= now {
      match OTPAuthUrl::parse(&self.otp_url) {
        Ok(otpauth) => {
          let (token, valid_until) = otpauth.generate(now);
          let mut token_display = self.base_view.find_id::<TextView>(&self.token_display_id).unwrap();
          token_display.set_content(token);
          self.valid_until = Some(valid_until);
          self.period = match otpauth.otp_type {
            OTPType::TOTP { period } => Some(u64::from(period)),
            _ => None,
          }
        }
        _ => (),
      }
    }
    if let Some(period) = self.period {
      let mut token_valid = self.base_view.find_id::<ProgressBar>(&self.token_valid_id).unwrap();
      if let Some(valid_until) = self.valid_until {
        token_valid.set_value((valid_until - now) as usize);
      }
    }

    Some(f(&mut self.base_view))
  }
}
