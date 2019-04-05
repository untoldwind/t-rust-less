use super::{open_store, Store, StoreError};
use rand::{thread_rng, Rng, ThreadRng};
use spectral::prelude::*;
use tempdir::TempDir;

fn common_store_tests(store: &mut Store) {
  let mut rng = thread_rng();
  common_test_ring(store, &mut rng);
  common_test_index(store, &mut rng);
  common_test_blocks(store, &mut rng);
}

fn common_test_ring(store: &mut Store, rng: &mut ThreadRng) {
  let ring1 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();
  let ring2 = rng.gen_iter::<u8>().take(300).collect::<Vec<u8>>();

  assert_that(&store.get_ring()).is_ok_containing(None);
  assert_that(&store.store_ring(&ring1)).is_ok();
  assert_that(&store.get_ring()).is_ok_containing(Some(ring1));
  assert_that(&store.store_ring(&ring2)).is_ok();
  assert_that(&store.get_ring()).is_ok_containing(Some(ring2));
}

fn common_test_index(store: &mut Store, rng: &mut ThreadRng) {
  let node1 = rng.gen_ascii_chars().take(40).collect::<String>();
  let node1_index1 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();
  let node1_index2 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();
  let node2 = rng.gen_ascii_chars().take(40).collect::<String>();
  let node2_index1 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();
  let node2_index2 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();

  assert_that(&store.get_index(&node1)).is_ok_containing(None);
  assert_that(&store.store_index(&node1, &node1_index1)).is_ok();
  assert_that(&store.get_index(&node2)).is_ok_containing(None);
  assert_that(&store.store_index(&node2, &node2_index1)).is_ok();
  assert_that(&store.get_index(&node1)).is_ok_containing(Some(node1_index1));
  assert_that(&store.get_index(&node2)).is_ok_containing(Some(node2_index1));
  assert_that(&store.store_index(&node1, &node1_index2)).is_ok();
  assert_that(&store.store_index(&node2, &node2_index2)).is_ok();
  assert_that(&store.get_index(&node1)).is_ok_containing(Some(node1_index2));
  assert_that(&store.get_index(&node2)).is_ok_containing(Some(node2_index2));
}

fn common_test_blocks(store: &mut Store, rng: &mut ThreadRng) {
  assert_that(&store.get_block("00000000000")).is_err_containing(StoreError::InvalidBlock("00000000000".to_string()));

  let block1 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();
  let block2 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();
  let block3 = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();

  let block1_id = store.add_block(&block1).unwrap();
  let block2_id = store.add_block(&block2).unwrap();
  let block3_id = store.add_block(&block3).unwrap();

  assert_that(&block1_id).is_not_equal_to(&block2_id);
  assert_that(&block1_id).is_not_equal_to(&block3_id);
  assert_that(&block2_id).is_not_equal_to(&block3_id);

  assert_that(&store.get_block(&block1_id)).is_ok_containing(block1);
  assert_that(&store.get_block(&block2_id)).is_ok_containing(block2);
  assert_that(&store.get_block(&block3_id)).is_ok_containing(block3);
}

#[test]
fn test_local_dir_store() {
  let tempdir = TempDir::new("t-rust-less-test").unwrap();
  let url = format!("file://{}", tempdir.path().to_string_lossy());

  let mut store = open_store(&url).unwrap();

  common_store_tests(store.as_mut());
}

#[test]
fn test_memory_store() {
  let mut store = open_store("memory://").unwrap();

  common_store_tests(store.as_mut());

}
