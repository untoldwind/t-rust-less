use super::hotp::HOTPGenerator;
use super::OTPAlgorithm;

#[derive(Debug)]
pub struct TOTPGenerator<'a> {
  pub algorithm: OTPAlgorithm,
  pub digits: u8,
  pub period: u32,
  pub secret: &'a [u8],
}

impl<'a> TOTPGenerator<'a> {
  pub fn generate(&self, timestamp: u64) -> (String, u64) {
    let mut hotp_gen = HOTPGenerator {
      algorithm: self.algorithm,
      counter: timestamp / u64::from(self.period),
      digits: self.digits,
      secret: self.secret,
    };
    (
      hotp_gen.generate().0,
      (timestamp / u64::from(self.period) + 1) * u64::from(self.period),
    )
  }
}
