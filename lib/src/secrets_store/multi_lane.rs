use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status};
use crate::memguard::SecretBytes;
use crate::secrets_store::cipher::{
  Cipher, KeyDerivation, PrivateKey, PublicKey, OPEN_SSL_RSA_AES_GCM, RUST_ARGON2_ID, RUST_X25519CHA_CHA20POLY1305,
};
use crate::secrets_store::{SecretStoreResult, SecretsStore};
use crate::secrets_store_capnp::{block, recipient, KeyType};
use std::sync::Mutex;

struct User {
  identity: Identity,
  public_keys: Vec<(KeyType, PublicKey)>,
  private_keys: Vec<(KeyType, PrivateKey)>,
}

pub struct MultiLaneSecretsStore {
  ciphers: Vec<&'static Cipher>,
  key_derivation: &'static KeyDerivation,
  unlocked_user: Mutex<Option<User>>,
}

impl MultiLaneSecretsStore {
  pub fn new() -> MultiLaneSecretsStore {
    MultiLaneSecretsStore {
      ciphers: vec![&OPEN_SSL_RSA_AES_GCM, &RUST_X25519CHA_CHA20POLY1305],
      key_derivation: &RUST_ARGON2_ID,
      unlocked_user: Mutex::new(None),
    }
  }
}

impl SecretsStore for MultiLaneSecretsStore {
  fn status() -> SecretStoreResult<Status> {
    unimplemented!()
  }

  fn lock() -> SecretStoreResult<()> {
    unimplemented!()
  }
  fn unlock(identity: &Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn identities() -> SecretStoreResult<Vec<Identity>> {
    unimplemented!()
  }
  fn add_identity(identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn list(filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    unimplemented!()
  }

  fn add(id: &str, secret_type: SecretType, secret_version: SecretVersion) -> SecretStoreResult<()> {
    unimplemented!()
  }
  fn get(id: &str) -> SecretStoreResult<Secret> {
    unimplemented!()
  }
}
