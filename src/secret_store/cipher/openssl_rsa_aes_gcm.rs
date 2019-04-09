use super::{Cipher, PrivateData, PrivateKey, PublicData, PublicKey, SealKey};
use crate::memguard::SecretBytes;
use crate::secret_store::{SecretStoreError, SecretStoreResult};
use crate::secret_store_capnp::{block, recipient};
use openssl::rsa::Rsa;
use openssl::symm;

const RSA_KEY_BITS: u32 = 4096;

const TAG_LENGTH: usize = 16;

pub struct OpenSslRsaAesGcmCipher;

impl Cipher for OpenSslRsaAesGcmCipher {
  fn generate_key_pair() -> SecretStoreResult<(PublicKey, PrivateKey)> {
    let private = Rsa::generate(RSA_KEY_BITS)?;
    let mut private_der_raw = private.private_key_to_der()?;
    let private_der = SecretBytes::from(private_der_raw.as_mut());
    let public_der = private.public_key_to_der()?;

    Ok((public_der, private_der))
  }

  fn seal_key_length() -> usize {
    32
  }

  fn seal_min_nonce_length() -> usize {
    12
  }

  fn seal_private_key(seal_key: &SealKey, nonce: &[u8], private_key: &PrivateKey) -> SecretStoreResult<PublicData> {
    let mut tag = [0u8; TAG_LENGTH];
    let mut result = symm::encrypt_aead(
      symm::Cipher::aes_256_gcm(),
      &seal_key.borrow(),
      Some(&nonce[0..12]),
      &[],
      &private_key.borrow(),
      &mut tag[..],
    )?;
    result.extend_from_slice(&tag[..]);

    Ok(result)
  }

  fn open_private_key(seal_key: &SealKey, nonce: &[u8], crypted_key: &PublicData) -> SecretStoreResult<PrivateKey> {
    if crypted_key.len() < TAG_LENGTH {
      return Err(SecretStoreError::Cipher("Data too short".to_string()));
    }
    let tag_offset = crypted_key.len() - TAG_LENGTH;
    let mut decrypted = symm::decrypt_aead(
      symm::Cipher::aes_256_gcm(),
      &seal_key.borrow(),
      Some(&nonce[0..12]),
      &[],
      &crypted_key[0..tag_offset],
      &crypted_key[tag_offset..],
    )?;

    Ok(SecretBytes::from(decrypted.as_mut()))
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
