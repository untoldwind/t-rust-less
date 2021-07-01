use super::{open_block_store, BlockStore, StoreError};
use crate::block_store::model::Operation;
use crate::block_store::{Change, ChangeLog};
use crate::memguard::weak::ZeroingWords;
use rand::rngs::ThreadRng;
use rand::{distributions, thread_rng, Rng};
use spectral::prelude::*;
use std::iter;
use std::sync::Arc;
use tempdir::TempDir;

fn common_store_tests(store: Arc<dyn BlockStore>) {
  let mut rng = thread_rng();
  common_test_ring(store.as_ref(), &mut rng);
  common_test_index(store.as_ref(), &mut rng);
  common_test_blocks_commits(store.as_ref(), &mut rng);
}

fn common_test_ring(store: &dyn BlockStore, rng: &mut ThreadRng) {
  let ring1a = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let ring1b = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(200)
    .collect::<Vec<u8>>();
  let ring2a = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(300 * 8)
    .collect::<Vec<u8>>();
  let ring2b = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(300 * 8)
    .collect::<Vec<u8>>();

  assert_that(&store.list_ring_ids()).is_ok_containing(vec![]);
  assert_that(&store.store_ring("ring1", &ring1a)).is_ok();
  assert_that(&store.get_ring("ring1")).is_ok_containing(ZeroingWords::from(ring1a.as_ref()));
  assert_that(&store.store_ring("ring1", &ring1b)).is_ok();
  assert_that(&store.get_ring("ring1")).is_ok_containing(ZeroingWords::from(ring1b.as_ref()));

  let mut ring_ids1 = store.list_ring_ids().unwrap();
  ring_ids1.sort();
  assert_that(&ring_ids1).is_equal_to(vec!["ring1".to_string()]);

  assert_that(&store.store_ring("ring2", &ring2a)).is_ok();
  assert_that(&store.get_ring("ring2")).is_ok_containing(ZeroingWords::from(ring2a.as_ref()));
  assert_that(&store.store_ring("ring2", &ring2b)).is_ok();
  assert_that(&store.get_ring("ring2")).is_ok_containing(ZeroingWords::from(ring2b.as_ref()));

  let mut ring_ids2 = store.list_ring_ids().unwrap();
  ring_ids2.sort();
  assert_that(&ring_ids2).is_equal_to(vec!["ring1".to_string(), "ring2".to_string()]);
}

fn common_test_index(store: &dyn BlockStore, rng: &mut ThreadRng) {
  let node1 = iter::repeat(())
    .map(|_| rng.sample(distributions::Alphanumeric))
    .map(char::from)
    .take(40)
    .collect::<String>();
  let node1_index1 = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let node1_index2 = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let node2 = iter::repeat(())
    .map(|_| rng.sample(distributions::Alphanumeric))
    .map(char::from)
    .take(40)
    .collect::<String>();
  let node2_index1 = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let node2_index2 = iter::repeat(())
    .map(|_| rng.sample(distributions::Standard))
    .take(200 * 8)
    .collect::<Vec<u8>>();

  assert_that(&store.get_index(&node1)).is_ok_containing(None);
  assert_that(&store.store_index(&node1, &node1_index1)).is_ok();
  assert_that(&store.get_index(&node2)).is_ok_containing(None);
  assert_that(&store.store_index(&node2, &node2_index1)).is_ok();
  assert_that(&store.get_index(&node1)).is_ok_containing(Some(ZeroingWords::from(node1_index1.as_ref())));
  assert_that(&store.get_index(&node2)).is_ok_containing(Some(ZeroingWords::from(node2_index1.as_ref())));
  assert_that(&store.store_index(&node1, &node1_index2)).is_ok();
  assert_that(&store.store_index(&node2, &node2_index2)).is_ok();
  assert_that(&store.get_index(&node1)).is_ok_containing(Some(ZeroingWords::from(node1_index2.as_ref())));
  assert_that(&store.get_index(&node2)).is_ok_containing(Some(ZeroingWords::from(node2_index2.as_ref())));
}

fn common_test_blocks_commits(store: &dyn BlockStore, rng: &mut ThreadRng) {
  assert_that(&store.get_block("00000000000")).is_err_containing(StoreError::InvalidBlock("00000000000".to_string()));

  let block1 = rng
    .sample_iter(&distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let block2 = rng
    .sample_iter(&distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();
  let block3 = rng
    .sample_iter(&distributions::Standard)
    .take(200 * 8)
    .collect::<Vec<u8>>();

  let block1_id = store.add_block(&block1).unwrap();
  let block2_id = store.add_block(&block2).unwrap();
  let block3_id = store.add_block(&block3).unwrap();

  assert_that(&block1_id).is_not_equal_to(&block2_id);
  assert_that(&block1_id).is_not_equal_to(&block3_id);
  assert_that(&block2_id).is_not_equal_to(&block3_id);

  assert_that(&store.get_block(&block1_id)).is_ok_containing(ZeroingWords::from(block1.as_ref()));
  assert_that(&store.get_block(&block2_id)).is_ok_containing(ZeroingWords::from(block2.as_ref()));
  assert_that(&store.get_block(&block3_id)).is_ok_containing(ZeroingWords::from(block3.as_ref()));

  assert_that(&store.commit(&[
    Change {
      op: Operation::Add,
      block: block1_id.clone(),
    },
    Change {
      op: Operation::Add,
      block: block2_id.clone(),
    },
  ]))
  .is_ok();

  assert_that(&store.change_logs()).is_ok_containing(vec![ChangeLog {
    node: store.node_id().to_string(),
    changes: vec![
      Change {
        op: Operation::Add,
        block: block1_id.clone(),
      },
      Change {
        op: Operation::Add,
        block: block2_id.clone(),
      },
    ],
  }]);

  assert_that(&store.commit(&[Change {
    op: Operation::Add,
    block: block2_id.clone(),
  }]))
  .is_err()
  .matches(|error| match error {
    StoreError::Conflict(_) => true,
    _ => false,
  });

  assert_that(&store.commit(&[Change {
    op: Operation::Add,
    block: block3_id.clone(),
  }]))
  .is_ok();

  assert_that(&store.change_logs()).is_ok_containing(vec![ChangeLog {
    node: store.node_id().to_string(),
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
  let tempdir = TempDir::new("t-rust-less-test").unwrap();
  let url = format!("file://{}", tempdir.path().to_string_lossy());

  let store = open_block_store(&url, "node1").unwrap();

  common_store_tests(store);
}

#[test]
fn test_memory_store() {
  let store = open_block_store("memory://", "node1").unwrap();

  common_store_tests(store);
}
