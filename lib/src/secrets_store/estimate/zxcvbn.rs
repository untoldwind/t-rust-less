use super::PasswordEstimator;
use crate::api::PasswordStrength;
use zxcvbn::time_estimates::CrackTimeSeconds;

pub struct ZxcvbnEstimator {}

impl PasswordEstimator for ZxcvbnEstimator {
  fn estimate_strength(password: &str, user_inputs: &[&str]) -> PasswordStrength {
    let entropy = zxcvbn::zxcvbn(password, user_inputs);
    PasswordStrength {
      entropy: (entropy.guesses() as f64).log2(),
      crack_time: match entropy.crack_times().offline_fast_hashing_1e10_per_second() {
        CrackTimeSeconds::Integer(i) => i as f64,
        CrackTimeSeconds::Float(f) => f,
      },
      crack_time_display: format!("{}", entropy.crack_times().offline_fast_hashing_1e10_per_second()),
      score: entropy.score() as u8,
    }
  }
}
