use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

use capnp::{message, serialize};

use crate::memguard::weak::ZeroingHeapAllocator;
use crate::memguard::SecretBytes;
use crate::secrets_store::cipher::{
  Cipher, KeyDerivation, PrivateKey, PublicKey, RUST_ARGON2_ID, RUST_X25519CHA_CHA20POLY1305,
};
use crate::secrets_store::estimate::{PasswordEstimator, ZxcvbnEstimator};
use crate::secrets_store::index::Index;
use crate::secrets_store::padding::{NonZeroPadding, Padding, RandomFrontBack};
use crate::secrets_store::{SecretStoreError, SecretStoreResult, SecretsStore};
use crate::secrets_store_capnp::{block, ring, KeyType};
use crate::{
  api::ZeroizeDateTime,
  block_store::{BlockStore, Change, Operation, StoreError},
};
use crate::{
  api::{Event, EventHub, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status},
  memguard::ZeroizeBytesBuffer,
};
use log::{info, warn};
use rand::{thread_rng, RngCore};
use std::collections::HashMap;

struct User {
  identity: Identity,
  public_keys: Vec<(KeyType, PublicKey)>,
  private_keys: Vec<(KeyType, PrivateKey)>,
  autolock_at: SystemTime,
  index: Index,
}

struct RecipientsForCipher<'a> {
  cipher: &'static dyn Cipher,
  recipient_keys: Vec<(&'a str, PublicKey)>,
}

pub struct MultiLaneSecretsStore {
  name: String,
  ciphers: Vec<&'static dyn Cipher>,
  key_derivation: &'static dyn KeyDerivation,
  unlocked_user: RwLock<Option<User>>,
  block_store: Arc<dyn BlockStore>,
  autolock_timeout: Duration,
  event_hub: Arc<dyn EventHub>,
}

impl MultiLaneSecretsStore {
  pub fn new(
    name: &str,
    block_store: Arc<dyn BlockStore>,
    autolock_timeout: Duration,
    event_hub: Arc<dyn EventHub>,
  ) -> MultiLaneSecretsStore {
    #[cfg(all(feature = "openssl", not(feature = "rust_crypto")))]
    let ciphers: Vec<&'static dyn Cipher> = vec![&super::cipher::OPEN_SSL_RSA_AES_GCM, &RUST_X25519CHA_CHA20POLY1305];
    #[cfg(feature = "rust_crypto")]
    let ciphers: Vec<&'static dyn Cipher> = vec![&super::cipher::RUST_RSA_AES_GCM, &RUST_X25519CHA_CHA20POLY1305];

    MultiLaneSecretsStore {
      name: name.to_string(),
      ciphers,
      key_derivation: &RUST_ARGON2_ID,
      unlocked_user: RwLock::new(None),
      block_store,
      autolock_timeout,
      event_hub,
    }
  }
}

impl SecretsStore for MultiLaneSecretsStore {
  fn status(&self) -> SecretStoreResult<Status> {
    let unlocked_user = self.unlocked_user.read()?;

    Ok(Status {
      locked: unlocked_user.is_none(),
      unlocked_by: unlocked_user.as_ref().map(|u| u.identity.clone()),
      autolock_at: unlocked_user.as_ref().map(|u| ZeroizeDateTime::from(u.autolock_at)),
      version: env!("CARGO_PKG_VERSION").to_string(),
      autolock_timeout: self.autolock_timeout.as_secs(),
    })
  }

  fn lock(&self) -> SecretStoreResult<()> {
    info!("Locking store");
    let mut unlocked_user = self.unlocked_user.write()?;
    unlocked_user.take();
    self.event_hub.send(Event::StoreLocked {
      store_name: self.name.clone(),
    });

    Ok(())
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let identity = {
      info!("Unlocking store for {}", identity_id);
      let mut unlocked_user = self.unlocked_user.write()?;

      if unlocked_user.is_some() {
        return Err(SecretStoreError::AlreadyUnlocked);
      }

      let mut raw: &[u8] = &self.block_store.get_ring(identity_id)?;
      let reader = serialize::read_message_from_flat_slice(&mut raw, Default::default())?;
      let ring = reader.get_root::<ring::Reader>()?;
      let mut private_keys = Vec::with_capacity(self.ciphers.len());
      let mut public_keys = Vec::with_capacity(self.ciphers.len());

      for user_private_key in ring.get_private_keys()? {
        if let Some(cipher) = self.find_cipher(user_private_key.get_type()?) {
          let nonce = user_private_key.get_nonce()?;
          if user_private_key.get_derivation_type()? != self.key_derivation.key_derivation_type() {
            return Err(SecretStoreError::KeyDerivation(
              "Key derivation method is not compatible".to_string(),
            ));
          }
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
      let index = self.read_index(identity_id, &private_keys)?;
      let identity = Self::identity_from_ring(ring)?;
      unlocked_user.replace(User {
        identity: identity.clone(),
        private_keys,
        public_keys,
        autolock_at: SystemTime::now() + self.autolock_timeout,
        index,
      });

      identity
    };

    self.update_index()?;

    self.event_hub.send(Event::StoreUnlocked {
      store_name: self.name.clone(),
      identity,
    });

    Ok(())
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    let ring_ids = self.block_store.list_ring_ids()?;
    let mut identities = Vec::with_capacity(ring_ids.len());

    for ring_id in ring_ids {
      let mut raw: &[u8] = &self.block_store.get_ring(&ring_id)?;
      let reader = serialize::read_message_from_flat_slice(&mut raw, Default::default())?;
      let ring = reader.get_root::<ring::Reader>()?;

      identities.push(Self::identity_from_ring(ring)?)
    }

    Ok(identities)
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    if self.block_store.list_ring_ids()?.iter().any(|id| id == &identity.id) {
      return Err(SecretStoreError::Conflict);
    }
    let mut ring_message = message::Builder::new(ZeroingHeapAllocator::default());
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
        user_private_key.set_derivation_type(self.key_derivation.key_derivation_type());
        user_private_key.set_preset(self.key_derivation.default_preset());
        user_private_key.set_nonce(&nonce);
        user_private_key.set_crypted_key(&crypted_key);
      }
    }
    let new_ring_raw = serialize::write_message_to_words(&ring_message);

    self.block_store.store_ring(&identity.id, &new_ring_raw)?;
    self.event_hub.send(Event::IdentityAdded {
      store_name: self.name.clone(),
      identity,
    });

    Ok(())
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

    let mut ring_message = message::Builder::new(ZeroingHeapAllocator::default());
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

    self.block_store.store_ring(&unlocked_user.identity.id, &new_ring_raw)?;

    Ok(())
  }

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

    unlocked_user.index.filter_entries(filter)
  }

  fn update_index(&self) -> SecretStoreResult<()> {
    let mut maybe_unlocked_user = self.unlocked_user.write()?;
    let unlocked_user = maybe_unlocked_user.as_mut().ok_or(SecretStoreError::Locked)?;
    let change_logs = self.block_store.change_logs()?;
    let identity_id = &unlocked_user.identity.id;
    let private_keys = &unlocked_user.private_keys;
    let index_updated = unlocked_user.index.process_change_logs(&change_logs, |block_id| {
      self.get_secret_version(identity_id, private_keys, block_id)
    })?;

    if index_updated {
      info!("Index has been updated");
      self.store_index(&unlocked_user.identity.id, &unlocked_user.index)?;
    }

    Ok(())
  }

  fn add(&self, mut secret_version: SecretVersion) -> SecretStoreResult<String> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

    if !secret_version
      .recipients
      .iter()
      .any(|recipient| unlocked_user.identity.id == recipient.as_str())
    {
      // User adding a secret version to the store is always a recipient
      secret_version.recipients.push(unlocked_user.identity.id.clone());
    }

    let block_content = {
      let mut buffer = ZeroizeBytesBuffer::with_capacity(1024);
      serde_json::to_writer(&mut buffer, &secret_version)?;

      self.ecnrypt_block(
        &secret_version.recipients,
        NonZeroPadding::pad_secret_data(&buffer, 512)?,
      )?
    };

    let block_id = self.block_store.add_block(&block_content)?;
    self.block_store.commit(&[Change {
      op: Operation::Add,
      block: block_id.clone(),
    }])?;
    self.event_hub.send(Event::SecretVersionAdded {
      store_name: self.name.clone(),
      secret_id: secret_version.secret_id.clone(),
      identity: unlocked_user.identity.clone(),
    });

    Ok(block_id)
  }

  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;
    let versions = unlocked_user.index.find_versions(secret_id)?;

    assert!(!versions.is_empty());

    let current_block_id = versions.first().unwrap().block_id.clone();
    let current = self
      .get_secret_version(
        &unlocked_user.identity.id,
        &unlocked_user.private_keys,
        &current_block_id,
      )?
      .ok_or(SecretStoreError::NotFound)?;
    let mut password_strengths = HashMap::with_capacity(current.secret_type.password_properties().len());

    for property in current.secret_type.password_properties() {
      if let Some(value) = current.properties.get(*property) {
        let strength = ZxcvbnEstimator::estimate_strength(value, &[&current.name, &unlocked_user.identity.name]);

        password_strengths.insert((*property).to_string(), strength);
      }
    }
    self.event_hub.send(Event::SecretOpened {
      store_name: self.name.clone(),
      secret_id: current.secret_id.clone(),
      identity: unlocked_user.identity.clone(),
    });

    Ok(Secret {
      id: current.secret_id.clone(),
      secret_type: current.secret_type,
      current,
      current_block_id,
      versions,
      password_strengths,
    })
  }

  fn get_version(&self, block_id: &str) -> SecretStoreResult<SecretVersion> {
    let maybe_unlocked_user = self.unlocked_user.read()?;
    let unlocked_user = maybe_unlocked_user.as_ref().ok_or(SecretStoreError::Locked)?;

    self
      .get_secret_version(&unlocked_user.identity.id, &unlocked_user.private_keys, block_id)?
      .ok_or(SecretStoreError::NotFound)
  }
}

impl MultiLaneSecretsStore {
  fn generate_nonce(len: usize) -> Vec<u8> {
    let mut rng = thread_rng();
    let mut nonce = vec![0u8; len];

    rng.fill_bytes(&mut nonce);

    nonce
  }

  fn find_cipher(&self, key_type: KeyType) -> Option<&'static dyn Cipher> {
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
      hidden: ring.get_hidden(),
    })
  }

  fn find_recipients<'a, T: AsRef<str>>(&self, recipients: &'a [T]) -> SecretStoreResult<Vec<RecipientsForCipher<'a>>> {
    let mut recipients_for_cipher: Vec<RecipientsForCipher<'a>> = self
      .ciphers
      .iter()
      .map(|cipher| RecipientsForCipher {
        cipher: *cipher,
        recipient_keys: Vec::with_capacity(recipients.len()),
      })
      .collect();

    for recipient in recipients {
      let identity_id = recipient.as_ref();
      let mut raw: &[u8] = &self.block_store.get_ring(identity_id).map_err(|e| match e {
        StoreError::InvalidBlock(_) => SecretStoreError::InvalidRecipient(identity_id.to_string()),
        err => err.into(),
      })?;
      let reader = serialize::read_message_from_flat_slice(&mut raw, Default::default())?;
      let ring = reader.get_root::<ring::Reader>()?;
      let user_public_keys = ring.get_public_keys()?;

      for RecipientsForCipher { cipher, recipient_keys } in recipients_for_cipher.iter_mut() {
        let key_type = cipher.key_type();
        let user_public_key = user_public_keys
          .iter()
          .find(|user_public_key| user_public_key.get_type() == Ok(key_type))
          .ok_or_else(|| {
            SecretStoreError::InvalidRecipient(format!("{} does not have required key type", identity_id))
          })?;

        recipient_keys.push((identity_id, user_public_key.get_key()?.to_vec()))
      }
    }

    Ok(recipients_for_cipher)
  }

  fn read_index(&self, identity_id: &str, private_keys: &[(KeyType, PrivateKey)]) -> SecretStoreResult<Index> {
    match self.block_store.get_index(identity_id)? {
      Some(crypted_index) => match self.decrypt_block(identity_id, private_keys, &crypted_index)? {
        Some(padded_index_data) => {
          let borrowed = padded_index_data.borrow();
          let index_data = RandomFrontBack::unpad_data(&borrowed)?;
          Ok(Index::from_secured_raw(index_data)?)
        }
        None => {
          warn!("User is not allowed recipient for index-data. Will trigger re-index.");
          Ok(Default::default())
        }
      },
      None => Ok(Default::default()),
    }
  }

  fn store_index(&self, identity_id: &str, index: &Index) -> SecretStoreResult<()> {
    let secret_content = RandomFrontBack::pad_secret_data(index.data.borrow().as_bytes(), 512)?;
    let block_content = self.ecnrypt_block(&[identity_id], secret_content)?;

    Ok(self.block_store.store_index(identity_id, &block_content)?)
  }

  fn get_secret_version(
    &self,
    identity_id: &str,
    private_keys: &[(KeyType, PrivateKey)],
    block_id: &str,
  ) -> SecretStoreResult<Option<SecretVersion>> {
    let block_words = self.block_store.get_block(block_id)?;

    match self.decrypt_block(identity_id, &private_keys, &block_words)? {
      Some(padded_content) => {
        let borrowed = padded_content.borrow();
        let version = serde_json::from_slice(NonZeroPadding::unpad_data(&borrowed)?)?;

        Ok(Some(version))
      }
      _ => Ok(None),
    }
  }

  fn ecnrypt_block<T: AsRef<str>>(
    &self,
    recipients: &[T],
    mut secret_content: SecretBytes,
  ) -> SecretStoreResult<Vec<u8>> {
    let recipients_for_cipher = self.find_recipients(recipients)?;
    let mut block_message = message::Builder::new(ZeroingHeapAllocator::default());
    let mut block = block_message.init_root::<block::Builder>();
    let mut headers = block.reborrow().init_headers(recipients_for_cipher.len() as u32);

    for (idx, RecipientsForCipher { cipher, recipient_keys }) in recipients_for_cipher.into_iter().enumerate() {
      let content = cipher.encrypt(&recipient_keys, &secret_content, headers.reborrow().get(idx as u32))?;

      secret_content = SecretBytes::from(content);
    }
    block.set_content(&secret_content.borrow());

    Ok(serialize::write_message_to_words(&block_message))
  }

  fn decrypt_block(
    &self,
    identity_id: &str,
    private_keys: &[(KeyType, PrivateKey)],
    mut block_words: &[u8],
  ) -> SecretStoreResult<Option<SecretBytes>> {
    let reader = serialize::read_message_from_flat_slice(&mut block_words, Default::default())?;
    let index_block = reader.get_root::<block::Reader>()?;
    let headers = index_block.reborrow().get_headers()?;

    if !Self::check_recipient(identity_id, &headers)? {
      return Ok(None);
    }

    let mut content = SecretBytes::from_secured(index_block.get_content()?);
    for idx in (0..headers.len()).rev() {
      let header = headers.get(idx);
      let cipher = self
        .find_cipher(header.get_type()?)
        .ok_or_else(|| SecretStoreError::Cipher("Unknown cipher".to_string()))?;
      let private_key = private_keys
        .iter()
        .find(|p| p.0 == cipher.key_type())
        .ok_or_else(|| SecretStoreError::MissingPrivateKey(cipher.name()))?;

      let next_content = cipher.decrypt((identity_id, &private_key.1), header, &content.borrow())?;
      content = next_content;
    }

    Ok(Some(content))
  }

  fn check_recipient<'a>(
    identity_id: &str,
    headers: &capnp::struct_list::Reader<'a, block::header::Owned>,
  ) -> SecretStoreResult<bool> {
    'outer: for header in headers.iter() {
      for recipient in header.get_recipients()? {
        if recipient.get_id()? == identity_id {
          continue 'outer;
        }
      }
      return Ok(false);
    }
    Ok(true)
  }
}

impl std::fmt::Debug for MultiLaneSecretsStore {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Multilane secrets store")
  }
}
