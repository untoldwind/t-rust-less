use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status};
use crate::block_store::BlockStore;
use crate::memguard::SecretBytes;
use crate::secrets_store::cipher::{
  Cipher, KeyDerivation, PrivateKey, PublicKey, OPEN_SSL_RSA_AES_GCM, RUST_ARGON2_ID, RUST_X25519CHA_CHA20POLY1305,
};
use crate::secrets_store::{SecretStoreResult, SecretsStore};
use crate::secrets_store_capnp::{block, recipient, KeyType};
use core::borrow::Borrow;
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
  block_store: Box<BlockStore>,
}

impl MultiLaneSecretsStore {
  pub fn new(block_store: Box<BlockStore>) -> MultiLaneSecretsStore {
    MultiLaneSecretsStore {
      ciphers: vec![&OPEN_SSL_RSA_AES_GCM, &RUST_X25519CHA_CHA20POLY1305],
      key_derivation: &RUST_ARGON2_ID,
      unlocked_user: Mutex::new(None),
      block_store,
    }
  }
}

impl SecretsStore for MultiLaneSecretsStore {
  fn status(&self) -> SecretStoreResult<Status> {
    let unlocked_user = self.unlocked_user.lock()?;
    unimplemented!()
  }

  fn lock(&mut self) -> SecretStoreResult<()> {
    unimplemented!()
  }
  fn unlock(&mut self, identity: &Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    unimplemented!()
  }
  fn add_identity(&mut self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    unimplemented!()
  }

  fn add(&mut self, id: &str, secret_type: SecretType, secret_version: SecretVersion) -> SecretStoreResult<()> {
    unimplemented!()
  }
  fn get(&self, id: &str) -> SecretStoreResult<Secret> {
    unimplemented!()
  }
}
