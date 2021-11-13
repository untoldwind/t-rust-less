use rand::{distributions, prelude::ThreadRng, thread_rng, Rng};
use spectral::prelude::*;
use std::{sync::Arc, time::Duration};

use crate::block_store::{open_block_store, BlockStore, Change, ChangeLog, Operation, RingId};

use super::SyncBlockStore;

fn sort_ring_ids(ring_ids: Vec<RingId>) -> Vec<String> {
  let mut ids: Vec<String> = ring_ids.into_iter().map(|(ring_id, _)| ring_id).collect();
  ids.sort();
  ids
}

fn test_ring_sync(
  rng: &mut ThreadRng,
  local_store: Arc<dyn BlockStore>,
  remote_store: Arc<dyn BlockStore>,
  sync_store: Arc<SyncBlockStore>,
) {
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

  assert_that!(local_store.store_ring("ring1a", 0, &ring1a)).is_ok();
  assert_that!(local_store.store_ring("ring1b", 0, &ring1b)).is_ok();

  assert_that!(remote_store.store_ring("ring2a", 0, &ring2a)).is_ok();
  assert_that!(remote_store.store_ring("ring2b", 0, &ring2b)).is_ok();

  assert_that!(local_store.list_ring_ids().map(sort_ring_ids))
    .is_ok_containing(vec!["ring1a".to_string(), "ring1b".to_string()]);
  assert_that!(remote_store.list_ring_ids().map(sort_ring_ids))
    .is_ok_containing(vec!["ring2a".to_string(), "ring2b".to_string()]);

  assert_that!(sync_store.synchronize()).is_ok();

  // Todo: Add assertions
}

fn test_block_sync(
  rng: &mut ThreadRng,
  local_store: Arc<dyn BlockStore>,
  remote_store: Arc<dyn BlockStore>,
  sync_store: Arc<SyncBlockStore>,
) {
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

  let block1_id = local_store.add_block(&block1).unwrap();

  let local_changes = vec![Change {
    op: Operation::Add,
    block: block1_id.clone(),
  }];
  assert_that!(local_store.commit(&local_changes)).is_ok();

  let block2_id = remote_store.add_block(&block2).unwrap();
  let block3_id = remote_store.add_block(&block3).unwrap();

  let remote_changes = vec![
    Change {
      op: Operation::Add,
      block: block2_id.clone(),
    },
    Change {
      op: Operation::Add,
      block: block3_id.clone(),
    },
  ];
  assert_that!(remote_store.commit(&remote_changes)).is_ok();

  assert_that!(local_store.get_block(&block1_id)).is_ok();
  assert_that!(local_store.get_block(&block2_id)).is_err();
  assert_that!(local_store.get_block(&block3_id)).is_err();

  assert_that!(remote_store.get_block(&block1_id)).is_err();
  assert_that!(remote_store.get_block(&block2_id)).is_ok();
  assert_that!(remote_store.get_block(&block3_id)).is_ok();

  assert_that!(sync_store.synchronize()).is_ok();

  assert_that!(local_store.get_block(&block1_id)).is_ok();
  assert_that!(local_store.get_block(&block2_id)).is_ok();
  assert_that!(local_store.get_block(&block3_id)).is_ok();

  assert_that!(remote_store.get_block(&block1_id)).is_ok();
  assert_that!(remote_store.get_block(&block2_id)).is_ok();
  assert_that!(remote_store.get_block(&block3_id)).is_ok();

  let expected_change_logs = vec![
    ChangeLog {
      node: "local".to_string(),
      changes: local_changes,
    },
    ChangeLog {
      node: "remote".to_string(),
      changes: remote_changes,
    },
  ];
  assert_that!(local_store.change_logs().map(|mut change_logs| {
    change_logs.sort_by(|a, b| a.node.cmp(&b.node));
    change_logs
  }))
  .is_ok_containing(&expected_change_logs);
  assert_that!(remote_store.change_logs().map(|mut change_logs| {
    change_logs.sort_by(|a, b| a.node.cmp(&b.node));
    change_logs
  }))
  .is_ok_containing(&expected_change_logs);
}

#[test]
fn test_memory_store() {
  let mut rng = thread_rng();
  let local_store = open_block_store("memory://", "local").unwrap();
  let remote_store = open_block_store("memory://", "remote").unwrap();
  let sync_store = Arc::new(SyncBlockStore::new(
    local_store.clone(),
    remote_store.clone(),
    Duration::from_secs(30),
  ));

  test_ring_sync(&mut rng, local_store.clone(), remote_store.clone(), sync_store.clone());
  test_block_sync(&mut rng, local_store, remote_store, sync_store);
}
