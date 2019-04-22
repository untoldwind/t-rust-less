use crate::api::{SecretType, SecretVersion};
use crate::block_store::{Change, ChangeLog, Operation};
use crate::memguard::weak::ZeroingStringExt;
use crate::secrets_store::index::Index;
use chrono::prelude::*;
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use spectral::prelude::*;
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
struct TestStore {
  versions: HashMap<String, SecretVersion>,
  changes: Vec<Change>,
}

impl TestStore {
  fn add_secret_version(&mut self, secret_id: &str, version_id: i64) {
    let block_id = Self::generate_block_id(secret_id, version_id);
    let version = Self::generate_secret_version(secret_id, version_id);

    self.versions.insert(block_id.clone(), version);
    self.changes.push(Change {
      op: Operation::Add,
      block: block_id,
    });
  }

  fn make_changelog(&self, node: &str) -> ChangeLog {
    ChangeLog {
      node: node.to_string(),
      changes: self.changes.clone(),
    }
  }

  fn generate_secret_version(secret_id: &str, version_id: i64) -> SecretVersion {
    SecretVersion {
      secret_id: secret_id.to_string(),
      secret_type: SecretType::Login,
      timestamp: Utc.timestamp(1000 + 1000 * version_id, 0),
      name: format!("{}_{}", secret_id, version_id).to_zeroing(),
      properties: BTreeMap::new(),
      tags: vec![],
      urls: vec![],
      deleted: false,
      recipients: vec![],
      attachments: vec![],
    }
  }

  fn generate_block_id(secret_id: &str, version_id: i64) -> String {
    let mut hasher = Sha256::new();

    hasher.input(secret_id);
    hasher.input(version_id.to_string());

    HEXLOWER.encode(&hasher.result())
  }
}

#[test]
fn test_process_change_logs() {
  let mut test_store: TestStore = Default::default();
  let mut index: Index = Default::default();

  for i in 0..10 {
    for j in 0..5 {
      test_store.add_secret_version(&format!("Secret_{}", i), j)
    }
  }

  assert_that(
    &index.process_change_logs(&[test_store.make_changelog("test_node")], |block_id| {
      Ok(test_store.versions.get(block_id).cloned())
    }),
  )
  .is_ok();

  let mut all_matches = index.filter_entries(Default::default()).unwrap();

  assert_that(&all_matches).has_length(10);

  test_store.changes.clear();

  for i in 10..15 {
    for j in 0..2 {
      test_store.add_secret_version(&format!("Secret_{}", i), j)
    }
  }

  assert_that(
    &index.process_change_logs(&[test_store.make_changelog("test_node")], |block_id| {
      Ok(test_store.versions.get(block_id).cloned())
    }),
  )
  .is_ok();

  all_matches = index.filter_entries(Default::default()).unwrap();

  assert_that(&all_matches).has_length(15);
}
