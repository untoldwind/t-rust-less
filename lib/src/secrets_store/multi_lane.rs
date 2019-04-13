use std::io::Cursor;
use std::sync::RwLock;
use std::time::SystemTime;

use capnp::{message, serialize};

use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status};
use crate::block_store::BlockStore;
use crate::memguard::SecretBytes;
use crate::secrets_store::cipher::{
  Cipher, KeyDerivation, PrivateKey, PublicKey, OPEN_SSL_RSA_AES_GCM, RUST_ARGON2_ID, RUST_X25519CHA_CHA20POLY1305,
};
use crate::secrets_store::{SecretStoreResult, SecretsStore};
use crate::secrets_store_capnp::{public_ring, KeyType};
use chrono::DateTime;

struct User {
  identity: Identity,
  public_keys: Vec<(KeyType, PublicKey)>,
  private_keys: Vec<(KeyType, PrivateKey)>,
  autolock_at: SystemTime,
}

pub struct MultiLaneSecretsStore {
  ciphers: Vec<&'static Cipher>,
  key_derivation: &'static KeyDerivation,
  unlocked_user: RwLock<Option<User>>,
  block_store: Box<BlockStore>,
}

impl MultiLaneSecretsStore {
  pub fn new(block_store: Box<BlockStore>) -> MultiLaneSecretsStore {
    MultiLaneSecretsStore {
      ciphers: vec![&OPEN_SSL_RSA_AES_GCM, &RUST_X25519CHA_CHA20POLY1305],
      key_derivation: &RUST_ARGON2_ID,
      unlocked_user: RwLock::new(None),
      block_store,
    }
  }
}

impl SecretsStore for MultiLaneSecretsStore {
  fn status(&self) -> SecretStoreResult<Status> {
    let unlocked_user = self.unlocked_user.read()?;
    let identities = self.identities()?;

    Ok(Status {
      initialized: !identities.is_empty(),
      locked: unlocked_user.is_none(),
      autolock_at: unlocked_user.as_ref().map(|u| DateTime::from(u.autolock_at)),
      version: env!("CARGO_PKG_VERSION").to_string(),
    })
  }

  fn lock(&mut self) -> SecretStoreResult<()> {
    let mut unlocked_user = self.unlocked_user.write()?;
    unlocked_user.take();

    Ok(())
  }

  fn unlock(&mut self, identity: &Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    match self.block_store.get_public_ring()? {
      Some(raw) => {
        let mut cursor = Cursor::new(&raw);
        let reader = serialize::read_message(&mut cursor, message::ReaderOptions::new())?;
        let public_ring = reader.get_root::<public_ring::Reader>()?;
        let recipients = public_ring.get_recipients()?;
        let mut identities = Vec::with_capacity(recipients.len() as usize);

        for recipient in recipients {
          identities.push(Identity {
            name: recipient.get_name()?.to_string(),
            email: recipient.get_email()?.to_string(),
          })
        }

        Ok(identities)
      }
      None => Ok(vec![]),
    }
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
