use std::io::Cursor;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use capnp::{message, serialize};

use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status};
use crate::block_store::BlockStore;
use crate::memguard::SecretBytes;
use crate::secrets_store::cipher::{
  Cipher, KeyDerivation, PrivateKey, PublicKey, OPEN_SSL_RSA_AES_GCM, RUST_ARGON2_ID, RUST_X25519CHA_CHA20POLY1305,
};
use crate::secrets_store::{SecretStoreError, SecretStoreResult, SecretsStore};
use crate::secrets_store_capnp::{ring, KeyType};
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
  block_store: Arc<BlockStore>,
  autolock_timeout: Duration,
}

impl MultiLaneSecretsStore {
  pub fn new(block_store: Arc<BlockStore>, autolock_timeout: Duration) -> MultiLaneSecretsStore {
    MultiLaneSecretsStore {
      ciphers: vec![&OPEN_SSL_RSA_AES_GCM, &RUST_X25519CHA_CHA20POLY1305],
      key_derivation: &RUST_ARGON2_ID,
      unlocked_user: RwLock::new(None),
      block_store,
      autolock_timeout,
    }
  }
}

impl SecretsStore for MultiLaneSecretsStore {
  fn status(&self) -> SecretStoreResult<Status> {
    let unlocked_user = self.unlocked_user.read()?;

    Ok(Status {
      locked: unlocked_user.is_none(),
      unlocked_by: unlocked_user.as_ref().map(|u| u.identity.clone()),
      autolock_at: unlocked_user.as_ref().map(|u| DateTime::from(u.autolock_at)),
      version: env!("CARGO_PKG_VERSION").to_string(),
    })
  }

  fn lock(&self) -> SecretStoreResult<()> {
    let mut unlocked_user = self.unlocked_user.write()?;
    unlocked_user.take();

    Ok(())
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let mut unlocked_user = self.unlocked_user.write()?;

    if unlocked_user.is_some() {
      return Err(SecretStoreError::AlreadUnlocked);
    }

    let raw = self.block_store.get_ring(identity_id)?;
    let mut cursor = Cursor::new(&raw);
    let reader = serialize::read_message(&mut cursor, message::ReaderOptions::new())?;
    let ring = reader.get_root::<ring::Reader>()?;
    let mut private_keys = Vec::with_capacity(self.ciphers.len());
    let mut public_keys = Vec::with_capacity(self.ciphers.len());

    for user_private_key in ring.get_private_keys()? {
      if let Some(cipher) = self.find_cipher(user_private_key.get_type()?) {
        let nonce = user_private_key.get_nonce()?;
        let seal_key = self.key_derivation.derive(
          &passphrase,
          user_private_key.get_preset(),
          nonce,
          cipher.seal_key_length(),
        )?;
        let private_key = cipher
          .open_private_key(&seal_key, nonce, user_private_key.get_crypted_key()?)
          .map_err(|_| SecretStoreError::InvalidPassphrase)?;

        private_keys.push((cipher.key_type(), private_key));
      }
    }
    for user_public_key in ring.get_public_keys()? {
      if let Some(cipher) = self.find_cipher(user_public_key.get_type()?) {
        public_keys.push((cipher.key_type(), user_public_key.get_key()?.to_vec()));
      }
    }
    unlocked_user.replace(User {
      identity: Self::identity_from_ring(ring)?,
      private_keys,
      public_keys,
      autolock_at: SystemTime::now() + self.autolock_timeout,
    });

    Ok(())
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    let ring_ids = self.block_store.list_ring_ids()?;
    let mut identities = Vec::with_capacity(ring_ids.len());

    for ring_id in ring_ids {
      let raw = self.block_store.get_ring(&ring_id)?;
      let mut cursor = Cursor::new(&raw);
      let reader = serialize::read_message(&mut cursor, message::ReaderOptions::new())?;
      let ring = reader.get_root::<ring::Reader>()?;

      identities.push(Self::identity_from_ring(ring)?)
    }

    Ok(identities)
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    if self.block_store.list_ring_ids()?.iter().any(|id| id == &identity.id) {
      return Err(SecretStoreError::Conflict);
    }
    let mut ring_message = message::Builder::new_default();
    let mut new_ring = ring_message.init_root::<ring::Builder>();

    new_ring.set_id(&identity.id);
    new_ring.set_name(&identity.name);
    new_ring.set_email(&identity.email);

    new_ring.reborrow().init_public_keys(self.ciphers.len() as u32);
    new_ring.reborrow().init_private_keys(self.ciphers.len() as u32);

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
        let mut user_public_key = new_ring.reborrow().get_public_keys()?.get(idx as u32);

        user_public_key.set_type(cipher.key_type());
        user_public_key.set_key(&public_key);
      }
      {
        let mut user_private_key = new_ring.reborrow().get_private_keys()?.get(idx as u32);

        user_private_key.set_type(cipher.key_type());
        user_private_key.set_preset(self.key_derivation.default_preset());
        user_private_key.set_nonce(&nonce);
        user_private_key.set_crypted_key(&crypted_key);
      }
    }
    let new_ring_raw = serialize::write_message_to_words(&ring_message);

    self
      .block_store
      .store_ring(&identity.id, capnp::Word::words_to_bytes(&new_ring_raw))?;

    Ok(())
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

    let mut ring_message = message::Builder::new_default();
    let mut new_ring = ring_message.init_root::<ring::Builder>();

    new_ring.set_id(&unlocked_user.identity.id);
    new_ring.set_name(&unlocked_user.identity.name);
    new_ring.set_email(&unlocked_user.identity.email);

    {
      let mut user_public_keys = new_ring.reborrow().init_public_keys(self.ciphers.len() as u32);
      for (idx, (key_type, public_key)) in unlocked_user.public_keys.iter().enumerate() {
        let mut user_public_key = user_public_keys.reborrow().get(idx as u32);

        user_public_key.set_type(*key_type);
        user_public_key.set_key(&public_key);
      }
    }

    let mut user_private_keys = new_ring.init_private_keys(self.ciphers.len() as u32);

    for (idx, (key_type, private_key)) in unlocked_user.private_keys.iter().enumerate() {
      let cipher = self
        .find_cipher(*key_type)
        .unwrap_or_else(|| panic!("Unlocked user with unknown cipher"));
      let nonce = Self::generate_nonce(cipher.seal_min_nonce_length().max(self.key_derivation.min_nonce_len()));
      let seal_key = self.key_derivation.derive(
        &passphrase,
        self.key_derivation.default_preset(),
        &nonce,
        cipher.seal_key_length(),
      )?;
      let crypted_key = cipher.seal_private_key(&seal_key, &nonce, &private_key)?;
      let mut user_private_key = user_private_keys.reborrow().get(idx as u32);

      user_private_key.set_type(cipher.key_type());
      user_private_key.set_preset(self.key_derivation.default_preset());
      user_private_key.set_nonce(&nonce);
      user_private_key.set_crypted_key(&crypted_key);
    }

    let new_ring_raw = serialize::write_message_to_words(&ring_message);

    self
      .block_store
      .store_ring(&unlocked_user.identity.id, capnp::Word::words_to_bytes(&new_ring_raw))?;

    Ok(())
  }

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

    unimplemented!()
  }

  fn add(&self, id: &str, secret_type: SecretType, secret_version: SecretVersion) -> SecretStoreResult<()> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

    unimplemented!()
  }

  fn get(&self, id: &str) -> SecretStoreResult<Secret> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

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

  fn find_cipher(&self, key_type: KeyType) -> Option<&'static Cipher> {
    for cipher in self.ciphers.iter() {
      if cipher.key_type() == key_type {
        return Some(*cipher);
      }
    }
    None
  }

  fn identity_from_ring(ring: ring::Reader) -> SecretStoreResult<Identity> {
    Ok(Identity {
      id: ring.get_id()?.to_string(),
      name: ring.get_name()?.to_string(),
      email: ring.get_email()?.to_string(),
    })
  }
}
