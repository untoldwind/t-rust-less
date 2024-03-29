use super::{Cipher, PrivateData, PrivateKey, PublicData, PublicKey, SealKey};
use crate::{memguard::SecretBytes, secrets_store::SecretStoreResult};
use crate::{
  secrets_store::SecretStoreError,
  secrets_store_capnp::{block, KeyType},
};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit};
use core::convert::TryFrom;
use rand::{thread_rng, RngCore};
use rsa::pkcs1::{DecodeRsaPrivateKey, EncodeRsaPrivateKey};
use rsa::pkcs8::{EncodePublicKey, SubjectPublicKeyInfo};
use rsa::{oaep::Oaep, RsaPrivateKey, RsaPublicKey};

const RSA_KEY_BITS: usize = 4096;

#[allow(dead_code)]
pub static RUST_RSA_AES_GCM: RustRsaAesGcmCipher = RustRsaAesGcmCipher();

pub struct RustRsaAesGcmCipher();

impl Cipher for RustRsaAesGcmCipher {
  fn key_type(&self) -> KeyType {
    KeyType::RsaAesGcm
  }

  fn name(&self) -> String {
    "RustRsaAesGcmCipher".to_string()
  }

  fn generate_key_pair(&self) -> SecretStoreResult<(PublicKey, PrivateKey)> {
    let mut rng = thread_rng();
    let private = RsaPrivateKey::new(&mut rng, RSA_KEY_BITS)?;
    let private_der = SecretBytes::from_secured(private.to_pkcs1_der()?.as_bytes());
    let public_der = private.to_public_key().to_public_key_der()?.as_ref().to_vec();

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
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&seal_key.borrow()));
    let encrypted = cipher.encrypt(GenericArray::from_slice(&nonce[0..12]), private_key.borrow().as_bytes())?;

    Ok(encrypted)
  }

  fn open_private_key(&self, seal_key: &SealKey, nonce: &[u8], crypted_key: &[u8]) -> SecretStoreResult<PrivateKey> {
    let cipher = Aes256Gcm::new(GenericArray::from_slice(&seal_key.borrow()));
    let decrypted = cipher.decrypt(GenericArray::from_slice(&nonce[0..12]), crypted_key)?;

    Ok(SecretBytes::from(decrypted))
  }

  fn encrypt(
    &self,
    recipients: &[(&str, PublicKey)],
    data: &PrivateData,
    mut header_builder: block::header::Builder,
  ) -> SecretStoreResult<PublicData> {
    let mut rng = thread_rng();
    let seal_key = SecretBytes::random(&mut rng, 32);
    let mut nonce = [0u8; 12];
    rng.fill_bytes(&mut nonce[..]);

    let cipher = Aes256Gcm::new(GenericArray::from_slice(&seal_key.borrow()));
    let public_data = cipher.encrypt(GenericArray::from_slice(&nonce[..]), data.borrow().as_bytes())?;

    header_builder.set_type(self.key_type());
    header_builder
      .reborrow()
      .init_common_key(12)
      .copy_from_slice(&nonce[..]);

    let mut recipient_keys = header_builder.init_recipients(recipients.len() as u32);

    for (idx, (recipient_id, recipient_public_key)) in recipients.iter().enumerate() {
      // Note: Parameter check in PKCS8 is slightly too strict and incompatible with previous versions, so this patch becomes necessary
      let mut s = SubjectPublicKeyInfo::try_from(recipient_public_key.as_ref())?;
      if s.algorithm.parameters.is_none() {
        s.algorithm.parameters = Some(rsa::pkcs1::der::asn1::Null.into());
      }
      let public_key = RsaPublicKey::try_from(s)?;

      let crypled_key_buffer = public_key.encrypt(&mut rng, Oaep::new::<sha1::Sha1>(), seal_key.borrow().as_bytes())?;

      let mut recipient_key = recipient_keys.reborrow().get(idx as u32);

      recipient_key.set_id(recipient_id);
      recipient_key
        .init_crypted_key(crypled_key_buffer.len() as u32)
        .copy_from_slice(&crypled_key_buffer);
    }

    Ok(public_data)
  }

  fn decrypt(
    &self,
    user: (&str, &PrivateKey),
    header: block::header::Reader,
    crypted: &[u8],
  ) -> SecretStoreResult<super::PrivateData> {
    if header.get_type()? != self.key_type() {
      return Err(SecretStoreError::Cipher("Invalid block header".to_string()));
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
      let private_key = RsaPrivateKey::from_pkcs1_der(&user.1.borrow())?;
      let seal_key = SecretBytes::from(private_key.decrypt(Oaep::new::<sha1::Sha1>(), crypted_key)?);

      let cipher = Aes256Gcm::new(GenericArray::from_slice(&seal_key.borrow()));
      let decrypted = cipher.decrypt(GenericArray::from_slice(&nonce[0..12]), crypted)?;

      return Ok(SecretBytes::from(decrypted));
    }

    Err(SecretStoreError::NoRecipient)
  }
}
