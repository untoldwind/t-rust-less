use super::PasswordEstimator;
use crate::api::PasswordStrength;
use zxcvbn::ZxcvbnError;

pub struct ZxcvbnEstimator {}

impl PasswordEstimator for ZxcvbnEstimator {
  fn estimate_strength(password: &str, user_inputs: &[&str]) -> PasswordStrength {
    match zxcvbn::zxcvbn(password, user_inputs) {
      Ok(entropy) => PasswordStrength {
        entropy: (entropy.guesses as f64).log2(),
        crack_time: entropy.crack_times_seconds.offline_fast_hashing_1e10_per_second,
        crack_time_display: entropy.crack_times_display.offline_fast_hashing_1e10_per_second,
        score: entropy.score,
      },
      Err(ZxcvbnError::BlankPassword) => PasswordStrength {
        entropy: 0.0,
        crack_time: 0.0,
        crack_time_display: "Instant".to_string(),
        score: 0,
      },
    }
  }
}
