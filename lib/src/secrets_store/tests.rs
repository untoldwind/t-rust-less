use super::{open_secrets_store, SecretStoreError, SecretStoreResult, SecretsStore};
use crate::api::{Identity, SecretType, SecretVersion};
use crate::memguard::weak::ZeroingStringExt;
use crate::memguard::SecretBytes;
use chrono::Utc;
use spectral::prelude::*;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

fn common_secrets_store_tests(secrets_store: Arc<SecretsStore>) {
  let initial_status = secrets_store.status().unwrap();

  assert_that(&initial_status.autolock_at).is_none();
  assert_that(&initial_status.locked).is_true();

  let initial_identities = secrets_store.identities().unwrap();

  assert_that(&initial_identities).is_empty();

  let ids_with_passphrase = add_identities_test(secrets_store.as_ref());

  add_secrets_versions(secrets_store.as_ref(), &ids_with_passphrase);
}

fn add_identities_test(secrets_store: &SecretsStore) -> Vec<(Identity, SecretBytes)> {
  let id1 = add_identity(secrets_store, "identity1", "Name1", "Email1", "Passphrase1").unwrap();
  let id2 = add_identity(secrets_store, "identity2", "Name2", "Email2", "Passphrase2").unwrap();

  let mut identities = secrets_store.identities().unwrap();
  identities.sort_by(|i1, i2| i1.id.cmp(&i2.id));

  assert_that(&identities).is_equal_to(vec![id1.clone(), id2.clone()]);

  assert_that(&add_identity(
    secrets_store,
    "identity1",
    "Name1",
    "Email1",
    "Passphrase1",
  ))
  .is_err_containing(SecretStoreError::Conflict);

  assert_that(&secrets_store.unlock("identity1", secret_from_str("Passphrase2")))
    .is_err_containing(SecretStoreError::InvalidPassphrase);

  secrets_store
    .unlock("identity1", secret_from_str("Passphrase1"))
    .unwrap();

  let unlock_status = secrets_store.status().unwrap();

  assert_that(&unlock_status.locked).is_false();
  assert_that(&unlock_status.unlocked_by).contains_value(id1.clone());

  secrets_store
    .change_passphrase(secret_from_str("Passphrase1abc"))
    .unwrap();

  secrets_store.lock().unwrap();

  let locked_status = secrets_store.status().unwrap();

  assert_that(&locked_status.locked).is_true();
  assert_that(&locked_status.unlocked_by).is_none();

  assert_that(&secrets_store.unlock("identity1", secret_from_str("Passphrase1")))
    .is_err_containing(SecretStoreError::InvalidPassphrase);

  secrets_store
    .unlock("identity1", secret_from_str("Passphrase1abc"))
    .unwrap();

  assert_that(&secrets_store.lock()).is_ok();

  vec![
    (id1, secret_from_str("Passphrase1abc")),
    (id2, secret_from_str("Passphrase2")),
  ]
}

fn add_secrets_versions(secrets_store: &SecretsStore, ids_with_passphrase: &[(Identity, SecretBytes)]) {
  let version1 = SecretVersion {
    secret_id: "secret1".to_string(),
    secret_type: SecretType::Login,
    timestamp: Utc::now(),
    name: "First secret".to_string().to_zeroing(),
    tags: vec![],
    urls: vec![],
    properties: BTreeMap::new(),
    attachments: vec![],
    deleted: false,
    recipients: ids_with_passphrase
      .iter()
      .map(|(id, _)| id.id.clone().to_zeroing())
      .collect(),
  };

  assert_that(&secrets_store.unlock(&ids_with_passphrase[0].0.id, ids_with_passphrase[0].1.clone())).is_ok();

  assert_that(&secrets_store.add(version1)).is_ok();
}

fn add_identity(
  secrets_store: &SecretsStore,
  id: &str,
  name: &str,
  email: &str,
  passphrase: &str,
) -> SecretStoreResult<Identity> {
  let id = Identity {
    id: id.to_string(),
    name: name.to_string(),
    email: email.to_string(),
  };

  secrets_store.add_identity(id.clone(), secret_from_str(passphrase))?;

  Ok(id)
}

fn secret_from_str(s: &str) -> SecretBytes {
  let mut raw = s.as_bytes().to_vec();

  SecretBytes::from(raw.as_mut())
}

#[test]
#[cfg_attr(debug_assertions, ignore)]
fn test_multi_lane_secrets_store() {
  let secrets_store = open_secrets_store("multilane+memory://", "node1", Duration::from_secs(300)).unwrap();

  common_secrets_store_tests(secrets_store)
}
