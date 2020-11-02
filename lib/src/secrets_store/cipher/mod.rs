use crate::memguard::SecretBytes;
use crate::secrets_store_capnp::{block, KeyDerivationType, KeyType};

use super::SecretStoreResult;

#[cfg(feature = "openssl")]
mod openssl_rsa_aes_gcm;
mod rust_argon2id;
#[cfg(feature = "rust_crypto")]
mod rust_rsa_aes_gcm;
mod rust_x25519_chacha20_poly1305;

#[cfg(feature = "openssl")]
pub use self::openssl_rsa_aes_gcm::OPEN_SSL_RSA_AES_GCM;
pub use self::rust_argon2id::RUST_ARGON2_ID;
#[cfg(feature = "rust_crypto")]
pub use self::rust_rsa_aes_gcm::RUST_RSA_AES_GCM;
pub use self::rust_x25519_chacha20_poly1305::RUST_X25519CHA_CHA20POLY1305;

#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod tests;

pub type PublicKey = Vec<u8>;
pub type PrivateKey = SecretBytes;
type PublicData = Vec<u8>;
type PrivateData = SecretBytes;
type SealKey = SecretBytes;

/// Common interface of all cipher suites.
///
/// In this case "Chiper" does not refer to a single cipher but rather to a set of
/// chiphers and methods used in combination to realize public/private key encryption
/// on data with multiple recipients.
///
pub trait Cipher: Send + Sync {
  /// Get the type identifier use inside the storage format.
  fn key_type(&self) -> KeyType;

  /// Get a displayable name of the cipher
  fn name(&self) -> String;

  /// Generate a new public-private key-pair.
  ///
  /// The cipher should decide by itself a suitable key-strength.
  ///
  fn generate_key_pair(&self) -> SecretStoreResult<(PublicKey, PrivateKey)>;

  /// Get the required length of the seal key for the `seal_private_key` and `open_private_key` operation.
  fn seal_key_length(&self) -> usize;

  /// Get the minimal nonce length for all seal/open/encrypt/decrypt operations.
  fn seal_min_nonce_length(&self) -> usize;

  /// Seal a private key of this cipher suite.
  ///
  /// * `seal_key` the sealing key created by a key-derivation, ensured to have exactly `seal_key_length` bytes
  /// * `nonce` random nonce to use, ensured to have at least `seal_min_nonce_length` bytes
  /// * `private_key` the private key to seal, created by a `generate_key_pair` of this suite
  ///
  fn seal_private_key(
    &self,
    seal_key: &SealKey,
    nonce: &[u8],
    private_key: &PrivateKey,
  ) -> SecretStoreResult<PublicData>;

  /// Open a sealed private key of this cipher suite.
  ///
  /// * `seal_key` the sealing key created by a key-derivation, ensured to have exactly `seal_key_length` bytes
  /// * `nonce` random nonce to use, ensured to have at least `seal_min_nonce_length` bytes
  /// * `crypted_key` the encrypted bytes created by a `seal_private_key`
  ///
  fn open_private_key(&self, seal_key: &SealKey, nonce: &[u8], crypted_key: &[u8]) -> SecretStoreResult<PrivateKey>;

  /// Encrypt arbitrary data for a set of recipients.
  ///
  /// * `recipients` list of recipients allowed to access/decrypt the data. It has to be
  ///   ensured that each recipient contains a public-key compatible with this suite.
  /// * `data` the data to encrypt
  /// * `header_builder` reference to the builder creating the encapsulating data-block for
  ///   storage
  ///
  fn encrypt(
    &self,
    recipients: &[(&str, PublicKey)],
    data: &PrivateData,
    header_builder: block::header::Builder,
  ) -> SecretStoreResult<PublicData>;

  /// Decrypt data for a user
  ///
  /// * `user` the user accessing/decrypting the data. It has to be ensured that the user
  ///   contains a private-key compatible with this suite and is part of the recipient list
  ///   of the data.
  /// * `header` reference to the header of the stored data-block.
  /// * `crypted` the encrypted data
  ///
  fn decrypt(
    &self,
    user: (&str, &PrivateKey),
    header: block::header::Reader,
    crypted: &[u8],
  ) -> SecretStoreResult<PrivateData>;

  fn find_matching_header<'a>(
    &self,
    headers: &capnp::struct_list::Reader<'a, block::header::Owned>,
  ) -> SecretStoreResult<Option<block::header::Reader<'a>>> {
    for header in headers.iter() {
      if header.get_type()? == self.key_type() {
        return Ok(Some(header));
      }
    }
    Ok(None)
  }
}

/// Common interface for a key-derivation method.
///
/// An implmentation of KeyDerivation is used to derive the seal-key of a Cipher.
///
/// Each method may have multiple presets for internal parameters that have to be adjusted to
/// common CPU power and use-case. Each preset is identified by a simple number.
///
pub trait KeyDerivation: Send + Sync {
  /// Get the key derivation type of the implmenetation.
  fn key_derivation_type(&self) -> KeyDerivationType;

  /// Get the default preset to use (for new keys).
  fn default_preset(&self) -> u8;

  /// Get the minmal length of a nonce for key-derivation.
  fn min_nonce_len(&self) -> usize;

  /// Derive a seal-key from a passphrase.
  ///
  /// * `passphrase` provided by the user
  /// * `preset` key-derivation preset to use
  /// * `nonce` random nonce to use, ensured to have at least `min_nonce_len` bytes
  /// * `key_length` the required key-length of the seal-key. The output must have exactly
  ///   this length.
  ///
  fn derive(&self, passphrase: &SecretBytes, preset: u8, nonce: &[u8], key_length: usize)
    -> SecretStoreResult<SealKey>;
}
