use super::{open_secrets_store, SecretsStore};
use crate::api::Identity;
use crate::memguard::SecretBytes;
use spectral::prelude::*;

fn common_secrets_store_tests(secrets_store: &mut SecretsStore) {
  let initial_status = secrets_store.status().unwrap();

  assert_that(&initial_status.autolock_at).is_none();
  assert_that(&initial_status.locked).is_true();

  let initial_identities = secrets_store.identities().unwrap();

  assert_that(&initial_identities).is_empty();

  let id1 = Identity {
    id: "identity1".to_string(),
    name: "Name1".to_string(),
    email: "Email1".to_string(),
  };
  let mut passphrase1 = b"Passphrase1".to_vec();
  secrets_store
    .add_identity(id1, SecretBytes::from(passphrase1.as_mut()))
    .unwrap();

  let identities = secrets_store.identities().unwrap();

  assert_that(&identities).has_length(1);
}

#[test]
#[cfg_attr(debug_assertions, ignore)]
fn test_multi_lane_secrets_store() {
  let mut secrets_store = open_secrets_store("multilane+memory://").unwrap();

  common_secrets_store_tests(secrets_store.as_mut())
}
