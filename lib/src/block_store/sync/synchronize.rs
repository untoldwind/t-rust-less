use std::{
  collections::{HashMap, HashSet},
  sync::Arc,
};

use log::info;

use crate::block_store::{BlockStore, Operation, StoreResult};

pub fn synchronize_rings(local: Arc<dyn BlockStore>, remote: Arc<dyn BlockStore>) -> StoreResult<bool> {
  let mut local_changes = false;
  let local_ring_ids: HashMap<String, u64> = local.list_ring_ids()?.into_iter().collect();
  let remote_ring_ids: HashMap<String, u64> = remote.list_ring_ids()?.into_iter().collect();

  for (remote_ring_id, remote_version) in remote_ring_ids.iter() {
    if let Some(local_version) = local_ring_ids.get(remote_ring_id) {
      if *remote_version <= *local_version {
        continue;
      }
    }
    info!("Downloading ring: {remote_ring_id}");
    let (remote_version, ring) = remote.get_ring(remote_ring_id)?;
    local.store_ring(remote_ring_id, remote_version, &ring)?;
    local_changes = true
  }

  for (local_ring_id, local_version) in local_ring_ids.iter() {
    if let Some(remote_version) = remote_ring_ids.get(local_ring_id) {
      if *local_version <= *remote_version {
        continue;
      }
    }
    info!("Uploading ring: {local_ring_id}");
    let (local_version, ring) = local.get_ring(local_ring_id)?;
    remote.store_ring(local_ring_id, local_version, &ring)?;
  }

  Ok(local_changes)
}

pub fn synchronize_blocks(local: Arc<dyn BlockStore>, remote: Arc<dyn BlockStore>) -> StoreResult<bool> {
  let mut local_changes = false;
  let local_change_logs = local.change_logs()?;
  let local_added: HashSet<&String> = local_change_logs
    .iter()
    .flat_map(|change_log| change_log.changes.iter())
    .filter_map(|change| match change.op {
      Operation::Add => Some(&change.block),
      _ => None,
    })
    .collect();
  let local_removed: HashSet<&String> = local_change_logs
    .iter()
    .flat_map(|change_log| change_log.changes.iter())
    .filter_map(|change| match change.op {
      Operation::Delete => Some(&change.block),
      _ => None,
    })
    .collect();
  let local_existing: HashSet<&String> = local_added.difference(&local_removed).copied().collect();
  let remote_change_logs = remote.change_logs()?;
  let remote_added: HashSet<&String> = remote_change_logs
    .iter()
    .flat_map(|change_log| change_log.changes.iter())
    .filter_map(|change| match change.op {
      Operation::Add => Some(&change.block),
      _ => None,
    })
    .collect();
  let remote_removed: HashSet<&String> = remote_change_logs
    .iter()
    .flat_map(|change_log| change_log.changes.iter())
    .filter_map(|change| match change.op {
      Operation::Delete => Some(&change.block),
      _ => None,
    })
    .collect();
  let remote_existing: HashSet<&String> = remote_added.difference(&remote_removed).copied().collect();

  for local_missing in remote_existing.difference(&local_existing).copied() {
    if local_removed.contains(local_missing) {
      continue;
    }
    info!("Downloading block: {local_missing}");
    let block = remote.get_block(local_missing)?;
    local.add_block(&block)?;
    local_changes = true;
  }

  for remote_missing in local_existing.difference(&remote_existing).copied() {
    if remote_removed.contains(remote_missing) {
      continue;
    }
    info!("Uploading block: {remote_missing}");
    let block = local.get_block(remote_missing)?;
    remote.add_block(&block)?;
  }

  Ok(local_changes)
}
