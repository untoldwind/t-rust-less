use super::{KeyDerivation, SealKey};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
use argon2::{self, Config, ThreadMode, Variant, Version};

pub static RUST_ARGON2_ID: RustArgon2id = RustArgon2id();

struct Preset {
  pub lanes: u32,
  pub mem_cost: u32,
  pub time_cost: u32,
  pub variant: Variant,
  pub version: Version,
}

const PRESETS: &[Preset] = &[Preset {
  lanes: 4,
  mem_cost: 64 * 1024,
  time_cost: 5,
  version: Version::Version13,
  variant: Variant::Argon2id,
}];

pub struct RustArgon2id();

impl KeyDerivation for RustArgon2id {
  fn default_preset(&self) -> u8 {
    0
  }

  fn min_nonce_len(&self) -> usize {
    8
  }

  fn derive(
    &self,
    passphrase: &SecretBytes,
    preset: u8,
    nonce: &[u8],
    key_length: usize,
  ) -> SecretStoreResult<SealKey> {
    let p = PRESETS
      .get(preset as usize)
      .ok_or_else(|| SecretStoreError::Cipher(format!("Invalid key derivation preset: {}", preset)))?;
    let config = Config {
      ad: &[],
      hash_length: key_length as u32,
      lanes: p.lanes,
      mem_cost: p.mem_cost,
      secret: &[],
      thread_mode: ThreadMode::default(),
      time_cost: p.time_cost,
      version: p.version,
      variant: p.variant,
    };

    let mut raw = argon2::hash_raw(&passphrase.borrow(), nonce, &config)?;

    Ok(SecretBytes::from(raw.as_mut()))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::memguard::SecretBytes;
  use data_encoding::HEXLOWER;
  use spectral::prelude::*;

  #[test]
  #[cfg_attr(debug_assertions, ignore)]
  fn test_derive_regression() {
    assert_that(&RUST_ARGON2_ID.min_nonce_len()).is_greater_than_or_equal_to(8);

    let mut passphrase_raw: Vec<u8> = Vec::from(&b"The password"[..]);
    let passphrase = SecretBytes::from(passphrase_raw.as_mut());

    // Regression: echo -n "The password" | argon2 12345678 -id -t 5 -m 16 -p 4 -v 13 -l 32
    assert_that!(HEXLOWER
      .encode(
        &RUST_ARGON2_ID
          .derive(&passphrase, RUST_ARGON2_ID.default_preset(), b"12345678", 32)
          .unwrap()
          .borrow()
      )
      .as_str())
    .is_equal_to("1179eb7e9e244e66010b245ca18da1191c00eaf45b724cd34b95c67219c01cc2");

    // Regression: echo -n "The password" | argon2 1234567812345678 -id -t 5 -m 16 -p 4 -v 13 -l 32
    assert_that!(HEXLOWER
      .encode(
        &RUST_ARGON2_ID
          .derive(&passphrase, RUST_ARGON2_ID.default_preset(), b"1234567812345678", 32)
          .unwrap()
          .borrow()
      )
      .as_str())
    .is_equal_to("cb537c1db49e0b24a302ddb7509dfa992071f5ba71099f41d71d0bdf1330a7e5");
  }
}