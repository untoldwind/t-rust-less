use super::{open_block_store, BlockStore, RingId, StoreError};
use crate::block_store::model::Operation;
use crate::block_store::{Change, ChangeLog, RingContent, StoreResult};
use crate::memguard::weak::ZeroingWords;
use data_encoding::HEXLOWER;
use rand::rngs::ThreadRng;
use rand::{distributions, thread_rng, Rng};
use spectral::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::Builder;

struct ExpectedState {
  rings: HashMap<String, RingContent>,
  change_logs: HashMap<String, Vec<Change>>,
  blocks: HashMap<String, ZeroingWords>,
}

impl ExpectedState {
  fn from_store(store: Arc<dyn BlockStore>) -> StoreResult<Self> {
    let mut rings = HashMap::new();
    for ring_id in store.list_ring_ids()? {
      let content = store.get_ring(&ring_id.0)?;
      rings.insert(ring_id.0, content);
    }
    let mut blocks = HashMap::new();
    let mut change_logs = HashMap::new();
    for change_log in store.change_logs()? {
      for change in &change_log.changes {
        if change.op == Operation::Add {
          blocks.insert(change.block.clone(), store.get_block(&change.block)?);
        }
      }
      change_logs.insert(change_log.node, change_log.changes);
    }

    Ok(ExpectedState {
      rings,
      change_logs,
      blocks,
    })
  }

  fn test(self, actual: Arc<dyn BlockStore>) {
    for (ring_id, expected) in self.rings {
      assert_that!(actual.get_ring(&ring_id)).is_ok_containing(expected);
    }
    for (block_id, expected) in self.blocks {
      assert_that!(actual.get_block(&block_id)).is_ok_containing(expected);
    }
    let mut actual_change_logs = HashMap::new();
    for change_log in actual.change_logs().unwrap() {
      actual_change_logs.insert(change_log.node, change_log.changes);
    }
    assert_that!(actual_change_logs).is_equal_to(self.change_logs);
  }
}

fn common_persistent_store_tests(url: &str) {
  let store = open_block_store(url, "node1").unwrap();

  common_store_tests(store.clone());

  let expected = ExpectedState::from_store(store).unwrap();
  let reopened = open_block_store(url, "node1").unwrap();
  expected.test(reopened);
}

fn common_store_tests(store: Arc<dyn BlockStore>) {
  let mut rng = thread_rng();
  common_test_ring(store.as_ref(), &mut rng);
  common_test_index(store.as_ref(), &mut rng);
  common_test_blocks_commits(store.as_ref(), &mut rng);
}

fn sort_ring_ids(ring_ids: Vec<RingId>) -> Vec<String> {
  let mut ids: Vec<String> = ring_ids
    .into_iter()
    .map(|(id, version)| format!("{}.{}", id, version))
    .collect();
  ids.sort();
  ids
}

fn common_test_ring(store: &dyn BlockStore, rng: &mut ThreadRng) {
  let ring1a = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let ring1b = rng.sample_iter(distributions::Standard).take(200).collect::<Vec<u8>>();
  let ring2a = rng
    .sample_iter(distributions::Standard)
    .take(300 * 8)
    .collect::<Vec<u8>>();
  let ring2b = rng
    .sample_iter(distributions::Standard)
    .take(300 * 8)
    .collect::<Vec<u8>>();

  assert_that!(store.list_ring_ids()).is_ok_containing(vec![]);
  assert_that!(store.store_ring("ring1", 0, &ring1a)).is_ok();
  assert_that!(store.get_ring("ring1")).is_ok_containing((0u64, ZeroingWords::from(ring1a.as_ref())));
  assert_that!(store.list_ring_ids().map(sort_ring_ids)).is_ok_containing(vec!["ring1.0".to_string()]);

  assert_that!(store.store_ring("ring1", 0, &ring1b)).is_err_containing(StoreError::Conflict(
    "Ring ring1 with version 0 already exists".to_string(),
  ));
  assert_that!(store.store_ring("ring1", 1, &ring1b)).is_ok();
  assert_that!(store.get_ring("ring1")).is_ok_containing((1u64, ZeroingWords::from(ring1b.as_ref())));

  assert_that!(store.list_ring_ids().map(sort_ring_ids)).is_ok_containing(vec!["ring1.1".to_string()]);

  assert_that!(store.store_ring("ring2", 0, &ring2a)).is_ok();
  assert_that!(store.get_ring("ring2")).is_ok_containing((0u64, ZeroingWords::from(ring2a.as_ref())));
  assert_that!(store.list_ring_ids().map(sort_ring_ids))
    .is_ok_containing(vec!["ring1.1".to_string(), "ring2.0".to_string()]);
  assert_that!(store.store_ring("ring2", 0, &ring2b)).is_err_containing(StoreError::Conflict(
    "Ring ring2 with version 0 already exists".to_string(),
  ));
  assert_that!(store.store_ring("ring2", 123, &ring2b)).is_ok();
  assert_that!(store.get_ring("ring2")).is_ok_containing((123u64, ZeroingWords::from(ring2b.as_ref())));

  assert_that!(store.list_ring_ids().map(sort_ring_ids))
    .is_ok_containing(vec!["ring1.1".to_string(), "ring2.123".to_string()]);
}

fn common_test_index(store: &dyn BlockStore, rng: &mut ThreadRng) {
  let node1 = rng
    .sample_iter(distributions::Alphanumeric)
    .map(char::from)
    .take(40)
    .collect::<String>();
  let node1_index1 = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let node1_index2 = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let node2 = rng
    .sample_iter(distributions::Alphanumeric)
    .map(char::from)
    .take(40)
    .collect::<String>();
  let node2_index1 = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let node2_index2 = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();

  assert_that!(store.get_index(&node1)).is_ok_containing(None);
  assert_that!(store.store_index(&node1, &node1_index1)).is_ok();
  assert_that!(store.get_index(&node2)).is_ok_containing(None);
  assert_that!(store.store_index(&node2, &node2_index1)).is_ok();
  assert_that!(store.get_index(&node1)).is_ok_containing(Some(ZeroingWords::from(node1_index1.as_ref())));
  assert_that!(store.get_index(&node2)).is_ok_containing(Some(ZeroingWords::from(node2_index1.as_ref())));
  assert_that!(store.store_index(&node1, &node1_index2)).is_ok();
  assert_that!(store.store_index(&node2, &node2_index2)).is_ok();
  assert_that!(store.get_index(&node1)).is_ok_containing(Some(ZeroingWords::from(node1_index2.as_ref())));
  assert_that!(store.get_index(&node2)).is_ok_containing(Some(ZeroingWords::from(node2_index2.as_ref())));
}

fn common_test_blocks_commits(store: &dyn BlockStore, rng: &mut ThreadRng) {
  let my_node_id = store.node_id();

  assert_that(&store.get_block("00000000000")).is_err_containing(StoreError::InvalidBlock("00000000000".to_string()));

  let non_existent = "does-not-exists";

  let block1 = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let block2 = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let block3 = rng
    .sample_iter(distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();

  let block3_id = HEXLOWER.encode(&rng.sample_iter(distributions::Standard).take(32).collect::<Vec<u8>>());

  let block1_id = store.add_block(&block1).unwrap();
  let block2_id = store.add_block(&block2).unwrap();
  store.insert_block(&block3_id, &my_node_id, &block3).unwrap();

  assert_that!(store.insert_block(&block1_id, &my_node_id, &block1))
    .is_err_containing(StoreError::Conflict(block1_id.to_string()));
  assert_that!(store.insert_block(&block2_id, &my_node_id, &block2))
    .is_err_containing(StoreError::Conflict(block2_id.to_string()));
  assert_that!(store.insert_block(&block3_id, &my_node_id, &block3))
    .is_err_containing(StoreError::Conflict(block3_id.to_string()));

  assert_that!(store.check_block(&block1_id)).is_ok_containing(true);
  assert_that!(store.check_block(&block2_id)).is_ok_containing(true);
  assert_that!(store.check_block(&block3_id)).is_ok_containing(true);
  assert_that!(store.check_block(&non_existent)).is_ok_containing(false);

  assert_that!(&block1_id).is_not_equal_to(&block2_id);
  assert_that!(&block1_id).is_not_equal_to(&block3_id);
  assert_that!(&block2_id).is_not_equal_to(&block3_id);

  assert_that!(store.get_block(&block1_id)).is_ok_containing(ZeroingWords::from(block1.as_ref()));
  assert_that!(store.get_block(&block2_id)).is_ok_containing(ZeroingWords::from(block2.as_ref()));
  assert_that!(store.get_block(&block3_id)).is_ok_containing(ZeroingWords::from(block3.as_ref()));

  assert_that(&store.change_logs()).is_ok_containing(vec![ChangeLog {
    node: my_node_id.to_string(),
    changes: vec![
      Change {
        op: Operation::Add,
        block: block1_id,
      },
      Change {
        op: Operation::Add,
        block: block2_id,
      },
      Change {
        op: Operation::Add,
        block: block3_id,
      },
    ],
  }]);
}

#[test]
fn test_local_dir_store() {
  let tempdir = Builder::new().prefix("t-rust-less-test").tempdir().unwrap();
  #[cfg(not(windows))]
  let url = format!("file://{}", tempdir.path().to_string_lossy());
  #[cfg(windows)]
  let url = format!("file:///{}", tempdir.path().to_string_lossy().replace('\\', "/"));

  common_persistent_store_tests(&url);
}

#[test]
fn test_memory_store() {
  let store = open_block_store("memory://", "node1").unwrap();

  common_store_tests(store);
}

#[test]
fn test_local_wal_store() {
  let tempdir = Builder::new().prefix("t-rust-less-test-wal").tempdir().unwrap();
  #[cfg(not(windows))]
  let url = format!("wal://{}", tempdir.path().to_string_lossy());
  #[cfg(windows)]
  let url = format!("wal:///{}", tempdir.path().to_string_lossy().replace('\\', "/"));

  common_persistent_store_tests(&url);
}

#[cfg(feature = "sled")]
#[test]
fn test_sled_store() {
  let tempdir = Builder::new().prefix("t-rust-less-test").tempdir().unwrap();
  #[cfg(not(windows))]
  let url = format!("sled://{}", tempdir.path().to_string_lossy());
  #[cfg(windows)]
  let url = format!("sled:///{}", tempdir.path().to_string_lossy().replace('\\', "/"));

  common_persistent_store_tests(&url);
}
