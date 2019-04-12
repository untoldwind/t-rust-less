use super::{Cipher, PrivateData, PrivateKey, PublicData, PublicKey, SealKey};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
use crate::secrets_store_capnp::{block, KeyType};
use capnp::data;
use openssl::rsa::{Padding, Rsa};
use openssl::symm;
use rand::{thread_rng, RngCore};

const RSA_KEY_BITS: u32 = 4096;

const TAG_LENGTH: usize = 16;

pub static OPEN_SSL_RSA_AES_GCM: OpenSslRsaAesGcmCipher = OpenSslRsaAesGcmCipher();
pub struct OpenSslRsaAesGcmCipher();

impl Cipher for OpenSslRsaAesGcmCipher {
  fn generate_key_pair(&self) -> SecretStoreResult<(PublicKey, PrivateKey)> {
    let private = Rsa::generate(RSA_KEY_BITS)?;
    let mut private_der_raw = private.private_key_to_der()?;
    let private_der = SecretBytes::from(private_der_raw.as_mut());
    let public_der = private.public_key_to_der()?;

    Ok((public_der, private_der))
  }

  fn seal_key_length(&self) -> usize {
    32
  }

  fn seal_min_nonce_length(&self) -> usize {
    12
  }

  fn seal_private_key(
    &self,
    seal_key: &SealKey,
    nonce: &[u8],
    private_key: &PrivateKey,
  ) -> SecretStoreResult<PublicData> {
    let mut tag = [0u8; TAG_LENGTH];
    let mut result = symm::encrypt_aead(
      symm::Cipher::aes_256_gcm(),
      &seal_key.borrow(),
      Some(&nonce[0..12]),
      &[],
      &private_key.borrow(),
      &mut tag,
    )?;
    result.extend_from_slice(&tag);

    Ok(result)
  }

  fn open_private_key(&self, seal_key: &SealKey, nonce: &[u8], crypted_key: &[u8]) -> SecretStoreResult<PrivateKey> {
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
    &self,
    recipients: &[(&str, &PublicKey)],
    data: &PrivateData,
    mut header_builder: block::header::Builder,
  ) -> SecretStoreResult<PublicData> {
    let mut rng = thread_rng();
    let mut tag = [0u8; TAG_LENGTH];
    let seal_key = SecretBytes::random(&mut rng, 32);
    let mut nonce = [0u8; 12];

    rng.fill_bytes(&mut nonce[..]);

    let mut public_data = symm::encrypt_aead(
      symm::Cipher::aes_256_gcm(),
      &seal_key.borrow(),
      Some(&nonce[..]),
      &[],
      &data.borrow(),
      &mut tag[..],
    )?;
    public_data.extend_from_slice(&tag);

    header_builder.set_type(KeyType::RsaAesGcm);
    header_builder
      .reborrow()
      .init_common_key(12)
      .copy_from_slice(&nonce[..]);

    let mut recipient_keys = header_builder.init_recipients(recipients.len() as u32);

    for (idx, (recipient_id, recipient_public_key)) in recipients.iter().enumerate() {
      let public_key = Rsa::public_key_from_der(recipient_public_key)?;
      let mut crypled_key_buffer = vec![0u8; public_key.size() as usize];

      let crypted_len = public_key.public_encrypt(&seal_key.borrow(), &mut crypled_key_buffer, Padding::PKCS1_OAEP)?;

      let mut recipient_key = recipient_keys.reborrow().get(idx as u32);

      recipient_key.set_id(recipient_id);
      recipient_key
        .init_crypted_key(crypted_len as u32)
        .copy_from_slice(&crypled_key_buffer[..crypted_len]);
    }

    Ok(public_data)
  }

  fn decrypt(
    &self,
    user: (&str, &PrivateKey),
    header: block::header::Reader,
    crypted: &[u8],
  ) -> SecretStoreResult<PrivateData> {
    if crypted.len() < TAG_LENGTH {
      return Err(SecretStoreError::Cipher("Data too short".to_string()));
    }
    let nonce = header.get_common_key()?;

    if nonce.len() != 12 {
      return Err(SecretStoreError::Cipher("Invalid nonce".to_string()));
    }

    for recipient in header.get_recipients()?.iter() {
      if user.0 != recipient.get_id()? {
        continue;
      }
      let crypted_key = recipient.get_crypted_key()?;
      let private_key = Rsa::private_key_from_der(&user.1.borrow())?;
      let mut seal_key = SecretBytes::zeroed(crypted_key.len());
      let seal_key_len =
        private_key.private_decrypt(&crypted_key, seal_key.borrow_mut().as_mut(), Padding::PKCS1_OAEP)?;

      if seal_key_len != 32 {
        return Err(SecretStoreError::Cipher("Decrypt seal key failed".to_string()));
      }

      let tag_offset = crypted.len() - TAG_LENGTH;
      let mut decrypted = symm::decrypt_aead(
        symm::Cipher::aes_256_gcm(),
        &seal_key.borrow()[..32],
        Some(nonce),
        &[],
        &crypted[0..tag_offset],
        &crypted[tag_offset..],
      )?;
      return Ok(SecretBytes::from(decrypted.as_mut()));
    }
    Err(SecretStoreError::NoRecipient)
  }
}
