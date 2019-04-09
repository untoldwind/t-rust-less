use super::{Cipher, PrivateData, PrivateKey, PublicData, PublicKey, SealKey};
use crate::secret_store::SecretStoreResult;
use crate::secret_store_capnp::{block, recipient};
use x25519_dalek::StaticSecret;
use rand::{OsRng, RngCore};
use crate::memguard::SecretBytes;
use chacha20_poly1305_aead::encrypt_read;
use std::io::Cursor;

pub struct RustX25519ChaCha20Poly1305Cipher;

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
    let private_borrow = private_key.borrow();
    let mut private_read = Cursor::new(private_borrow.as_ref());
    let tag = encrypt_read(
      &seal_key.borrow(),
      nonce,
      &[],
      &mut private_read,
      &mut result,
    )?;
    result.extend_from_slice(&tag[..]);

    Ok(result)
  }

  fn open_private_key(seal_key: &SealKey, nonce: &[u8], crypted_key: &PublicData) -> SecretStoreResult<PrivateKey> {
    unimplemented!()
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
