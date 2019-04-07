use super::SecretStoreResult;
use crate::memguard::SecretBytes;
use crate::secret_store_capnp::{block, recipient};

type PublicKey = Vec<u8>;
type PrivateKey = SecretBytes;
type PublicData = Vec<u8>;
type PrivateData = SecretBytes;

pub trait Cipher {
  fn generate_key_pair() -> SecretStoreResult<(PublicKey, PrivateKey)>;

  fn encrypt(
    recipients: &[(&str, &PublicKey)],
    data: &PrivateData,
  ) -> SecretStoreResult<(block::header::Owned, PublicData)>;

  fn decrypt(
    user: (&str, &PrivateKey),
    header: block::header::Reader,
    crypted: PublicData,
  ) -> SecretStoreResult<PrivateData>;
}
