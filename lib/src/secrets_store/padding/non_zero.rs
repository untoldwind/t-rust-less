use super::Padding;
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
use rand::{thread_rng, RngCore};
use std::io::Write;

/// Padding scheme that requires the padded data to contain no zero byte.
///
/// This come handy when storing plain ascii or json data.
///
/// The data is head and tail padded with random bytes, i.e. there is some junk in at
/// the start and some junk at the end so that an attacker cannot make assumptions about
/// the encrypted data (e.g. that a json always starts with a `{"` and ends with a `}`.
///
/// The outcome of the padding should look like this:
/// ```plain
///   <junk without \0> \0 <content> \0 <just junk with or without \0>
/// ```
/// The exact head and tail size is choosen at random depending on the size of the content.
///
pub struct NonZeroPadding;

impl Padding for NonZeroPadding {
  fn pad_secret_data(data: SecretBytes, align: usize) -> SecretStoreResult<SecretBytes> {
    assert!(data.borrow().iter().find(|b| **b == 0).is_none());

    let mut rng = thread_rng();
    let over_align = data.len() % align;

    if over_align == 0 {
      return Ok(data);
    }

    let mut pad_length = align - over_align - 1;

    if pad_length == 0 {
      pad_length = align
    }

    let mut pad_bytes = vec![0u8; pad_length];

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
