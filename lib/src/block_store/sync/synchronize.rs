use std::{collections::HashSet, sync::Arc};

use log::info;

use crate::block_store::{BlockStore, ChangeLog, Operation, StoreResult};

pub fn synchronize_blocks(local: Arc<dyn BlockStore>, remote: Arc<dyn BlockStore>) -> StoreResult<()> {
  let local_change_log = local
    .change_logs()?
    .into_iter()
    .find(|change_log| change_log.node == local.node_id())
    .unwrap_or_else(|| ChangeLog::new(local.node_id()));
  let local_added: HashSet<&String> = local_change_log
    .changes
    .iter()
    .filter_map(|change| match change.op {
      Operation::Add => Some(&change.block),
      _ => None,
    })
    .collect();
  let local_removed: HashSet<&String> = local_change_log
    .changes
    .iter()
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
    info!("Downloading block: {}", local_missing);
    let block = remote.get_block(local_missing)?;
    local.add_block(&block)?;
  }

  for remote_missing in local_existing.difference(&remote_existing).copied() {
    if remote_removed.contains(remote_missing) {
      continue;
    }
    info!("Uploading block: {}", remote_missing);
    let block = local.get_block(remote_missing)?;
    remote.add_block(&block)?;
  }

  for remote_change_log in remote_change_logs {
    if remote_change_log.node != local.node_id() {
      local.update_change_log(remote_change_log)?;
    }
  }

  remote.update_change_log(local_change_log)?;

  Ok(())
}
