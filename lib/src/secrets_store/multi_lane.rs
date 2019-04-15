use std::io::Cursor;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};

use capnp::{message, serialize};

use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretType, SecretVersion, Status};
use crate::block_store::BlockStore;
use crate::memguard::SecretBytes;
use crate::secrets_store::cipher::{
  Cipher, KeyDerivation, PrivateKey, PublicKey, OPEN_SSL_RSA_AES_GCM, RUST_ARGON2_ID, RUST_X25519CHA_CHA20POLY1305,
};
use crate::secrets_store::{SecretStoreError, SecretStoreResult, SecretsStore};
use crate::secrets_store_capnp::{public_ring, recipient, ring, KeyType};
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

    match self.block_store.get_ring()? {
      Some(raw) => {
        let mut cursor = Cursor::new(&raw);
        let reader = serialize::read_message(&mut cursor, message::ReaderOptions::new())?;
        let ring = reader.get_root::<ring::Reader>()?;
        let user = Self::find_user(ring, identity_id)?.ok_or(SecretStoreError::Forbidden)?;
        let mut private_keys = Vec::with_capacity(self.ciphers.len());
        let mut public_keys = Vec::with_capacity(self.ciphers.len());

        for user_private_key in user.get_private_keys()? {
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
        for recipient_public_key in user.get_recipient()?.get_public_keys()? {
          if let Some(cipher) = self.find_cipher(recipient_public_key.get_type()?) {
            public_keys.push((cipher.key_type(), recipient_public_key.get_key()?.to_vec()));
          }
        }
        unlocked_user.replace(User {
          identity: Self::identity_from_recipient(user.get_recipient()?)?,
          private_keys,
          public_keys,
          autolock_at: SystemTime::now() + self.autolock_timeout,
        });

        Ok(())
      }
      _ => Err(SecretStoreError::Forbidden),
    }
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
          identities.push(Self::identity_from_recipient(recipient)?)
        }

        Ok(identities)
      }
      None => Ok(vec![]),
    }
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
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
          if user.get_recipient()?.get_id()? == &identity.id {
            return Err(SecretStoreError::Conflict);
          }
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

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let unlocked_user = self.unlocked_user.read()?;

    if unlocked_user.is_none() {
      return Err(SecretStoreError::Locked);
    }

    unimplemented!()
  }

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    let unlocked_user = self.unlocked_user.read()?;

    if unlocked_user.is_none() {
      return Err(SecretStoreError::Locked);
    }

    unimplemented!()
  }

  fn add(&self, id: &str, secret_type: SecretType, secret_version: SecretVersion) -> SecretStoreResult<()> {
    let unlocked_user = self.unlocked_user.read()?;

    if unlocked_user.is_none() {
      return Err(SecretStoreError::Locked);
    }

    unimplemented!()
  }

  fn get(&self, id: &str) -> SecretStoreResult<Secret> {
    let unlocked_user = self.unlocked_user.read()?;

    if unlocked_user.is_none() {
      return Err(SecretStoreError::Locked);
    }

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

  fn find_user<'a>(ring: ring::Reader<'a>, id: &str) -> SecretStoreResult<Option<ring::user::Reader<'a>>> {
    for user in ring.get_users()? {
      let recipient = user.get_recipient()?;
      if recipient.get_id()? == id {
        return Ok(Some(user));
      }
    }
    Ok(None)
  }

  fn find_cipher(&self, key_type: KeyType) -> Option<&'static Cipher> {
    for cipher in self.ciphers.iter() {
      if cipher.key_type() == key_type {
        return Some(*cipher);
      }
    }
    None
  }

  fn identity_from_recipient(recipient: recipient::Reader) -> SecretStoreResult<Identity> {
    Ok(Identity {
      id: recipient.get_id()?.to_string(),
      name: recipient.get_name()?.to_string(),
      email: recipient.get_email()?.to_string(),
    })
  }
}
