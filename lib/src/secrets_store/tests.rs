use super::{open_secrets_store, SecretStoreError, SecretStoreResult, SecretsStore};
use crate::api::Identity;
use crate::memguard::SecretBytes;
use spectral::prelude::*;
use std::sync::Arc;
use std::time::Duration;

fn common_secrets_store_tests(secrets_store: Arc<SecretsStore>) {
  let initial_status = secrets_store.status().unwrap();

  assert_that(&initial_status.autolock_at).is_none();
  assert_that(&initial_status.locked).is_true();

  let initial_identities = secrets_store.identities().unwrap();

  assert_that(&initial_identities).is_empty();

  let id1 = add_identity(secrets_store.as_ref(), "identity1", "Name1", "Email1", "Passphrase1").unwrap();
  let id2 = add_identity(secrets_store.as_ref(), "identity2", "Name2", "Email2", "Passphrase2").unwrap();

  let identities = secrets_store.identities().unwrap();

  assert_that(&identities).has_length(2);
  assert_that(identities.get(0).unwrap()).is_equal_to(id1.clone());
  assert_that(identities.get(1).unwrap()).is_equal_to(id2);

  assert_that(&add_identity(
    secrets_store.as_ref(),
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
  assert_that(&unlock_status.unlocked_by).contains_value(id1);

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
  let secrets_store = open_secrets_store("multilane+memory://", Duration::from_secs(300)).unwrap();

  common_secrets_store_tests(secrets_store)
}
