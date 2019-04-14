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
use crate::secrets_store_capnp::{public_ring, ring, KeyType};
use chrono::DateTime;
use rand::{thread_rng, RngCore};

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

    Ok(Status {
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
            id: recipient.get_id()?.to_string(),
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
    let mut ring_message = message::Builder::new_default();
    let new_ring = ring_message.init_root::<ring::Builder>();

    let mut user = match self.block_store.get_ring()? {
      Some(raw) => {
        let mut cursor = Cursor::new(&raw);
        let reader = serialize::read_message(&mut cursor, message::ReaderOptions::new())?;
        let existing_ring = reader.get_root::<ring::Reader>()?;
        let existing_users = existing_ring.get_users()?;
        let users = new_ring.init_users(existing_users.len() + 1);

        for (idx, user) in existing_users.into_iter().enumerate() {
          users.set_with_caveats(idx as u32, user)?;
        }

        users.get(existing_users.len())
      }
      None => {
        let users = new_ring.init_users(1);

        users.get(0)
      }
    };
    {
      let mut recipient = user.reborrow().get_recipient()?;

      recipient.set_id(&identity.id);
      recipient.set_name(&identity.name);
      recipient.set_email(&identity.email);

      recipient.init_public_keys(self.ciphers.len() as u32);
    }
    user.reborrow().init_private_keys(self.ciphers.len() as u32);

    for (idx, cipher) in self.ciphers.iter().enumerate() {
      let (public_key, private_key) = cipher.generate_key_pair()?;
      let nonce = Self::generate_nonce(cipher.seal_min_nonce_length().max(self.key_derivation.min_nonce_len()));
      let seal_key = self.key_derivation.derive(
        &passphrase,
        self.key_derivation.default_preset(),
        &nonce,
        cipher.seal_key_length(),
      )?;
      let crypted_key = cipher.seal_private_key(&seal_key, &nonce, &private_key)?;

      {
        let mut recipient_key = user.reborrow().get_recipient()?.get_public_keys()?.get(idx as u32);

        recipient_key.set_type(cipher.key_type());
        recipient_key.set_key(&public_key);
      }
      {
        let mut user_key = user.reborrow().get_private_keys()?.get(idx as u32);

        user_key.set_type(cipher.key_type());
        user_key.set_preset(self.key_derivation.default_preset());
        user_key.set_nonce(&nonce);
        user_key.set_crypted_key(&crypted_key);
      }
    }
    let new_ring_raw = serialize::write_message_to_words(&ring_message);
    let new_public_ring_raw = Self::public_ring_from_private(&ring_message.get_root_as_reader()?)?;

    self
      .block_store
      .store_ring(capnp::Word::words_to_bytes(&new_ring_raw))?;
    self
      .block_store
      .store_public_ring(capnp::Word::words_to_bytes(&new_public_ring_raw))?;

    Ok(())
  }

  fn change_passphrase(&mut self, passphrase: SecretBytes) -> SecretStoreResult<()> {
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

impl MultiLaneSecretsStore {
  fn generate_nonce(len: usize) -> Vec<u8> {
    let mut rng = thread_rng();
    let mut nonce = vec![0u8; len];

    rng.fill_bytes(&mut nonce);

    nonce
  }

  fn public_ring_from_private(ring: &ring::Reader) -> SecretStoreResult<Vec<capnp::Word>> {
    let mut public_ring_message = message::Builder::new_default();
    let public_ring = public_ring_message.init_root::<public_ring::Builder>();
    let users = ring.get_users()?;
    let recipients = public_ring.init_recipients(users.len());

    for (idx, user) in users.into_iter().enumerate() {
      let recipient = user.get_recipient()?;

      recipients.set_with_caveats(idx as u32, recipient)?;
    }

    Ok(serialize::write_message_to_words(&public_ring_message))
  }
}
