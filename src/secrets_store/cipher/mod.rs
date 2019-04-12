use super::SecretStoreResult;
use crate::memguard::SecretBytes;
use crate::secrets_store_capnp::{block, recipient};

mod openssl_rsa_aes_gcm;
mod rust_argon2i;
mod rust_x25519_chacha20_poly1305;

#[cfg(test)]
mod tests;

type PublicKey = Vec<u8>;
type PrivateKey = SecretBytes;
type PublicData = Vec<u8>;
type PrivateData = SecretBytes;
type SealKey = SecretBytes;

pub trait Cipher {
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
  fn min_nonce_len(&self) -> usize;

  fn derive(&self, passphrase: &SecretBytes, nonce: &[u8], key_length: usize) -> SecretStoreResult<SealKey>;
}
