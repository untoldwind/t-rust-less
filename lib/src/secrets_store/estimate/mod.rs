use crate::api::PasswordStrength;

mod zxcvbn;

pub use self::zxcvbn::*;

pub trait PasswordEstimator {
  fn estimate_strength(password: &str, user_inputs: &[&str]) -> PasswordStrength;
}
