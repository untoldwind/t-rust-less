use super::{KeyDerivation, SealKey};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
use crate::secrets_store_capnp::KeyDerivationType;
use argon2::{self, Config, Variant, Version};

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
  time_cost: 4,
  version: Version::Version13,
  variant: Variant::Argon2id,
}];

pub struct RustArgon2id();

impl KeyDerivation for RustArgon2id {
  fn key_derivation_type(&self) -> KeyDerivationType {
    KeyDerivationType::Argon2
  }

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
      .ok_or_else(|| SecretStoreError::Cipher(format!("Invalid key derivation preset: {preset}")))?;
    let config = Config {
      ad: &[],
      hash_length: key_length as u32,
      thread_mode: argon2::ThreadMode::Sequential,
      lanes: p.lanes,
      mem_cost: p.mem_cost,
      secret: &[],
      time_cost: p.time_cost,
      version: p.version,
      variant: p.variant,
    };

    let raw = argon2::hash_raw(&passphrase.borrow(), nonce, &config)?;

    Ok(SecretBytes::from(raw))
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

    let passphrase = SecretBytes::from(Vec::from(&b"The password"[..]));

    // Regression: echo -n "The password" | argon2 12345678 -id -t 4 -m 16 -p 4 -v 13 -l 32
    assert_that!(HEXLOWER
      .encode(
        &RUST_ARGON2_ID
          .derive(&passphrase, RUST_ARGON2_ID.default_preset(), b"12345678", 32)
          .unwrap()
          .borrow()
      )
      .as_str())
    .is_equal_to("45942b82c50c93f9656369030480dfb83475f22663371dfd523f4893d062b493");

    // Regression: echo -n "The password" | argon2 1234567812345678 -id -t 4 -m 16 -p 4 -v 13 -l 32
    assert_that!(HEXLOWER
      .encode(
        &RUST_ARGON2_ID
          .derive(&passphrase, RUST_ARGON2_ID.default_preset(), b"1234567812345678", 32)
          .unwrap()
          .borrow()
      )
      .as_str())
    .is_equal_to("51b1dff59e6bece75db4a2f668622fb110098841820dfded0f724d42cb7dbdd2");
  }
}
