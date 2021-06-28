use super::PasswordEstimator;
use crate::api::PasswordStrength;
use zxcvbn::time_estimates::CrackTimeSeconds;
use zxcvbn::ZxcvbnError;

pub struct ZxcvbnEstimator {}

impl PasswordEstimator for ZxcvbnEstimator {
  fn estimate_strength(password: &str, user_inputs: &[&str]) -> PasswordStrength {
    match zxcvbn::zxcvbn(password, user_inputs) {
      Ok(entropy) => PasswordStrength {
        entropy: (entropy.guesses() as f64).log2(),
        crack_time: match entropy.crack_times().offline_fast_hashing_1e10_per_second() {
          CrackTimeSeconds::Integer(i) => i as f64,
          CrackTimeSeconds::Float(f) => f,
        },
        crack_time_display: format!("{}", entropy.crack_times().offline_fast_hashing_1e10_per_second()),
        score: entropy.score(),
      },
      Err(ZxcvbnError::BlankPassword) | Err(ZxcvbnError::DurationOutOfRange) => PasswordStrength {
        entropy: 0.0,
        crack_time: 0.0,
        crack_time_display: "Instant".to_string(),
        score: 0,
      },
    }
  }
}
