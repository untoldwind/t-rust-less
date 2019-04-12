use super::Padding;
use crate::memguard::SecretBytes;
use crate::secret_store::{SecretStoreError, SecretStoreResult};
use rand::{CryptoRng, RngCore};
use std::io::Write;

pub struct NonZeroPadding;

impl Padding for NonZeroPadding {
  fn pad_secret_data<T: RngCore + CryptoRng>(
    rng: &mut T,
    data: SecretBytes,
    align: usize,
  ) -> SecretStoreResult<SecretBytes> {
    let over_align = data.len() % align;

    if over_align == 0 {
      return Ok(data);
    }

    let mut pad_bytes = vec![0u8; align - over_align - 1];

    rng.fill_bytes(&mut pad_bytes);

    let first_zero = match pad_bytes.iter().position(|b| *b == 0) {
      Some(random_zero) => random_zero,
      None => {
        let pos = rng.next_u64() as usize % pad_bytes.len();
        pad_bytes[pos] = 0;
        pos
      }
    };
    let mut padded_data = SecretBytes::with_capacity(data.len() + pad_bytes.len() + 1);

    {
      let mut padded_writer = padded_data.borrow_mut();

        padded_writer.write_all(&pad_bytes[..first_zero + 1])?;
        padded_writer.write_all(&data.borrow())?;
        padded_writer.write_all(&pad_bytes[first_zero..])?;
    }

    Ok(padded_data)
  }

  fn unpad_data(padded: &[u8]) -> SecretStoreResult<&[u8]> {
    match padded.iter().position(|b| *b == 0) {
      Some(first_zero) => match padded[first_zero + 1..].iter().position(|b| *b == 0) {
        Some(next_zero) => Ok(&padded[first_zero + 1..first_zero + next_zero + 1]),
        None => Err(SecretStoreError::Padding),
      },
      None => Ok(padded),
    }
  }
}
