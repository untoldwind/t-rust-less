use super::SecretStoreResult;
use crate::memguard::SecretBytes;

mod non_zero;
#[cfg(test)]
mod tests;

pub use self::non_zero::*;
use rand::{CryptoRng, RngCore};

pub trait Padding {
  fn pad_secret_data<T: RngCore + CryptoRng>(
    rng: &mut T,
    data: SecretBytes,
    align: usize,
  ) -> SecretStoreResult<SecretBytes>;

  fn unpad_data(padded: &[u8]) -> SecretStoreResult<&[u8]>;
}
