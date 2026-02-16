use std::{os::unix::fs::MetadataExt, path::Path};

use tokio::{fs, sync::mpsc::Sender};

use crate::{
  error::SyncResult,
  layout::{Layout, LocalFile, SyncProgress},
  remote_fs::RemoteFS,
};

pub struct LocalDirLayout<'a, P, R> {
  client_id: &'a str,
  remote_fs: R,
  local_path: P,
  sync_progress: Sender<SyncProgress>,
  step: u32,
  remaining: u32,
}

impl<'a, P: AsRef<Path> + Send, R: RemoteFS> LocalDirLayout<'a, P, R> {
  async fn read_local_dir(&self, sub_dir: &str) -> SyncResult<Vec<LocalFile>> {
    let mut result = vec![];
    let mut read_dir = fs::read_dir(self.local_path.as_ref().join(sub_dir)).await?;
    while let Some(entry) = read_dir.next_entry().await? {
      let file_type = entry.file_type().await?;
      if !file_type.is_file() {
        continue;
      }
      let name = entry.file_name().to_string_lossy().to_string();
      let metadata = entry.metadata().await?;
      result.push(LocalFile {
        path: self.local_path.as_ref().join(sub_dir).join(&name),
        name: name,
        size: metadata.size(),
        mtime: metadata.modified()?,
      });
    }
    Ok(result)
  }

  async fn sync_remote_ring_to_local(&mut self) -> SyncResult<()> {
    self.step += 1;
    self.remaining -= 1;
    self
      .sync_progress
      .send(SyncProgress {
        step: self.step,
        remaining: self.remaining,
        message: "Sync ring: remote -> local".to_string(),
      })
      .await?;
    let remote_files = self.remote_fs.list_folder("rings").await?;
    let local_files = self.read_local_dir("rings").await?;

    todo!()
  }
}

impl<'a, P: AsRef<Path> + Send + Sync, R: RemoteFS> Layout<P, R> for LocalDirLayout<'a, P, R> {
  async fn sync_remote_to_local(
    client_id: &str,
    remote_fs: R,
    local_path: P,
    sync_progress: Sender<SyncProgress>,
  ) -> SyncResult<()> {
    let mut layout = LocalDirLayout {
      client_id,
      remote_fs,
      local_path,
      sync_progress,
      step: 1,
      remaining: 2,
    };
    layout.sync_remote_ring_to_local().await?;
    todo!()
  }
}
