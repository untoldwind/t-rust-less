use crate::memguard::SecretBytes;
use crate::secrets_store_capnp::{block, KeyType};

use super::SecretStoreResult;

mod openssl_rsa_aes_gcm;
mod rust_argon2id;
mod rust_x25519_chacha20_poly1305;

pub use self::openssl_rsa_aes_gcm::OPEN_SSL_RSA_AES_GCM;
pub use self::rust_argon2id::RUST_ARGON2_ID;
pub use self::rust_x25519_chacha20_poly1305::RUST_X25519CHA_CHA20POLY1305;

#[cfg(test)]
mod tests;

pub type PublicKey = Vec<u8>;
pub type PrivateKey = SecretBytes;
type PublicData = Vec<u8>;
type PrivateData = SecretBytes;
type SealKey = SecretBytes;

pub trait Cipher: Send + Sync {
  fn key_type(&self) -> KeyType;

  fn generate_key_pair(&self) -> SecretStoreResult<(PublicKey, PrivateKey)>;

  fn seal_key_length(&self) -> usize;

  fn seal_min_nonce_length(&self) -> usize;

  fn seal_private_key(
    &self,
    seal_key: &SealKey,
    nonce: &[u8],
    private_key: &PrivateKey,
  ) -> SecretStoreResult<PublicData>;

  fn open_private_key(&self, seal_key: &SealKey, nonce: &[u8], crypted_key: &[u8]) -> SecretStoreResult<PrivateKey>;

  fn encrypt(
    &self,
    recipients: &[(&str, &PublicKey)],
    data: &PrivateData,
    header_builder: block::header::Builder,
  ) -> SecretStoreResult<PublicData>;

  fn decrypt(
    &self,
    user: (&str, &PrivateKey),
    header: block::header::Reader,
    crypted: &[u8],
  ) -> SecretStoreResult<PrivateData>;
}

pub trait KeyDerivation {
  fn default_preset(&self) -> u8;

  fn min_nonce_len(&self) -> usize;

  fn derive(&self, passphrase: &SecretBytes, preset: u8, nonce: &[u8], key_length: usize)
    -> SecretStoreResult<SealKey>;
}
