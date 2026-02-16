use std::{
  path::{Path, PathBuf},
  time::SystemTime,
};

use crate::{error::SyncResult, remote_fs::RemoteFS};

mod local_dir;
pub use local_dir::*;
use tokio::sync::mpsc::Sender;

pub struct SyncProgress {
  pub step: u32,
  pub remaining: u32,
  pub message: String,
}

pub struct LocalFile {
  pub name: String,
  pub path: PathBuf,
  pub size: u64,
  pub mtime: SystemTime,
}

pub trait Layout<P: AsRef<Path> + Send + Sync, R: RemoteFS> {
  fn sync_remote_to_local(
    client_id: &str,
    remote_fs: R,
    local_path: P,
    sync_progress: Sender<SyncProgress>,
  ) -> impl std::future::Future<Output = SyncResult<()>> + Send;
}
