use super::SecretStoreResult;
use crate::memguard::SecretBytes;

mod non_zero;
mod random_front_back;
#[cfg(test)]
mod tests;

pub use self::non_zero::*;
pub use self::random_front_back::*;

pub trait Padding {
  fn pad_secret_data(data: &[u8], align: usize) -> SecretStoreResult<SecretBytes>;

  fn unpad_data(padded: &[u8]) -> SecretStoreResult<&[u8]>;
}
