use crate::memguard::SecretBytes;
use crate::secrets_store::padding::Padding;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
use rand::{thread_rng, RngCore};
use std::io::Write;

/// Padding scheme similar to NonZeroPadding without any requirements to the padded
/// data.
///
/// The outcome of the padding should look like this:
/// ```plain
///   <junk without \0> \0 <7-bit encoded length of content> <content> <just junk with or without \0>
/// ```
pub struct RandomFrontBack;

impl Padding for RandomFrontBack {
  fn pad_secret_data(data: &[u8], align: usize) -> SecretStoreResult<SecretBytes> {
    let encoded_length = Self::encode_length(data.len());
    let effective_length = data.len() + 1 + encoded_length.len();
    let pad_length = align - (effective_length % align);
    let mut pad_bytes = vec![0u8; pad_length];
    let (head_pad, tail_pad) = if pad_bytes.is_empty() {
      (&pad_bytes[..], &pad_bytes[..])
    } else {
      let mut rng = thread_rng();

      rng.fill_bytes(&mut pad_bytes);

      match pad_bytes.iter().position(|b| *b == 0) {
        Some(random_zero) => (&pad_bytes[0..random_zero], &pad_bytes[random_zero..]),
        None => {
          let pos = rng.next_u64() as usize % pad_bytes.len();
          (&pad_bytes[0..pos], &pad_bytes[pos..])
        }
      }
    };
    let mut padded_data = SecretBytes::with_capacity(effective_length + head_pad.len() + tail_pad.len());

    {
      let mut padded_writer = padded_data.borrow_mut();

      padded_writer.write_all(head_pad)?;
      padded_writer.write_all(&[0u8])?;
      padded_writer.write_all(&encoded_length)?;
      padded_writer.write_all(&data)?;
      padded_writer.write_all(tail_pad)?;
    }

    Ok(padded_data)
  }

  fn unpad_data(padded: &[u8]) -> SecretStoreResult<&[u8]> {
    match padded.iter().position(|b| *b == 0) {
      Some(pos) if pos < padded.len() - 1 => {
        let (offset, length) = Self::decode_length(&padded[pos + 1..])?;

        if pos + 1 + offset + length > padded.len() {
          Err(SecretStoreError::Padding)
        } else {
          Ok(&padded[pos + 1 + offset..pos + 1 + offset + length])
        }
      }
      _ => Err(SecretStoreError::Padding),
    }
  }
}

impl RandomFrontBack {
  fn encode_length(mut length: usize) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(8);
    loop {
      let b = (length & 0x7f) as u8;
      length >>= 7;
      encoded.push(b);
      if length == 0 {
        break;
      }
    }
    encoded.reverse();
    let prefix = encoded.len() - 1;
    for b in &mut encoded[0..prefix] {
      *b |= 0x80;
    }

    encoded
  }

  fn decode_length(bytes: &[u8]) -> SecretStoreResult<(usize, usize)> {
    let mut offset = 0;
    let mut length = 0usize;

    loop {
      if offset >= bytes.len() {
        return Err(SecretStoreError::Padding);
      }
      let b = bytes[offset];

      length <<= 7;
      length |= (b & 0x7f) as usize;
      offset += 1;

      if b & 0x80 == 0 {
        break;
      }
    }
    Ok((offset, length))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use quickcheck::quickcheck;
  use spectral::prelude::*;

  #[test]
  fn test_encode_length() {
    assert_that(&RandomFrontBack::encode_length(0)).is_equal_to(vec![0u8]);
    assert_that(&RandomFrontBack::encode_length(0x12)).is_equal_to(vec![0x12u8]);
    assert_that(&RandomFrontBack::encode_length(0x7f)).is_equal_to(vec![0x7fu8]);
    assert_that(&RandomFrontBack::encode_length(0x80)).is_equal_to(vec![0x81u8, 0x0u8]);
    assert_that(&RandomFrontBack::encode_length(0x1234)).is_equal_to(vec![0xa4u8, 0x34u8]);

    assert_that(&RandomFrontBack::decode_length(&[0u8])).is_ok_containing((1, 0));
    assert_that(&RandomFrontBack::decode_length(&[0x12u8])).is_ok_containing((1, 0x12));
    assert_that(&RandomFrontBack::decode_length(&[0x7fu8])).is_ok_containing((1, 0x7f));
    assert_that(&RandomFrontBack::decode_length(&[0x81u8, 0x0u8])).is_ok_containing((2, 0x80));
    assert_that(&RandomFrontBack::decode_length(&[0xa4u8, 0x34u8])).is_ok_containing((2, 0x1234));
  }

  #[test]
  fn test_encode_length_quick() {
    fn check_encode_decode(length: usize) -> bool {
      let encoded = RandomFrontBack::encode_length(length);

      if encoded.is_empty() {
        return false;
      }
      let (offset, actual) = RandomFrontBack::decode_length(&encoded).unwrap();

      offset == encoded.len() && actual == length
    }
    quickcheck(check_encode_decode as fn(usize) -> bool);
  }
}
