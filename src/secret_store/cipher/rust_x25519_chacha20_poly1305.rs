use super::{Cipher, PrivateData, PrivateKey, PublicData, PublicKey, SealKey};
use crate::secret_store::SecretStoreResult;
use crate::secret_store_capnp::{block, recipient};

struct RustX25519ChaCha20Poly1305Cipher;

impl Cipher for RustX25519ChaCha20Poly1305Cipher {
  fn generate_key_pair() -> SecretStoreResult<(PublicKey, PrivateKey)> {
    unimplemented!()
  }

  fn seal_key_length() -> usize {
    unimplemented!()
  }

  fn seal_min_nonce_length() -> usize {
    unimplemented!()
  }

  fn seal_private_key(seal_key: &SealKey, nonce: &[u8], private_key: &PrivateKey) -> SecretStoreResult<PublicData> {
    unimplemented!()
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
