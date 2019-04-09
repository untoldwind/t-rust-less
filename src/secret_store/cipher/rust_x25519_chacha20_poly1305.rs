use super::{Cipher, PrivateData, PrivateKey, PublicData, PublicKey, SealKey};
use crate::memguard::SecretBytes;
use crate::secret_store::{SecretStoreError, SecretStoreResult};
use crate::secret_store_capnp::{block, recipient};
use chacha20_poly1305_aead::{decrypt, encrypt};
use rand::{OsRng, RngCore};
use std::io::Cursor;
use x25519_dalek::StaticSecret;

pub struct RustX25519ChaCha20Poly1305Cipher;

const TAG_LENGTH: usize = 16;

impl Cipher for RustX25519ChaCha20Poly1305Cipher {
  fn generate_key_pair() -> SecretStoreResult<(PublicKey, PrivateKey)> {
    let mut rng = OsRng::new().unwrap();
    let private = StaticSecret::new(&mut rng);
    let public = x25519_dalek::PublicKey::from(&private);
    let mut private_raw = private.to_bytes();

    Ok((public.as_bytes().to_vec(), SecretBytes::from(&mut private_raw[..])))
  }

  fn seal_key_length() -> usize {
    32
  }

  fn seal_min_nonce_length() -> usize {
    12
  }

  fn seal_private_key(seal_key: &SealKey, nonce: &[u8], private_key: &PrivateKey) -> SecretStoreResult<PublicData> {
    let mut result = Vec::with_capacity(private_key.len());
    let tag = encrypt(&seal_key.borrow(), nonce, &[], &private_key.borrow(), &mut result)?;
    result.extend_from_slice(&tag[..]);

    Ok(result)
  }

  fn open_private_key(seal_key: &SealKey, nonce: &[u8], crypted_key: &PublicData) -> SecretStoreResult<PrivateKey> {
    if crypted_key.len() < TAG_LENGTH {
      return Err(SecretStoreError::Cipher("Data too short".to_string()));
    }
    let tag_offset = crypted_key.len() - TAG_LENGTH;
    let mut result = SecretBytes::with_capacity(crypted_key.len() - TAG_LENGTH);
    decrypt(
      &seal_key.borrow(),
      nonce,
      &[],
      &crypted_key[0..tag_offset],
      &crypted_key[tag_offset..],
      &mut result.borrow_mut(),
    )?;

    Ok(result)
  }

  fn encrypt(
    recipients: &[(&str, &PublicKey)],
    data: &PrivateData,
  ) -> SecretStoreResult<(block::header::Owned, PublicData)> {
    unimplemented!()
  }

  fn decrypt(
    user: (&str, &PrivateKey),
    header: block::header::Reader,
    crypted: PublicData,
  ) -> SecretStoreResult<PrivateData> {
    unimplemented!()
  }
}
