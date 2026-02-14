use std::path::Path;

use crate::{error::SyncResult, remote_fs::RemoteFS};

mod local_dir;
pub use local_dir::*;

pub struct SyncProgress {
  pub step: u32,
  pub remaining: u32,
  pub message: String,
}

pub trait Layout {
  fn sync_remote_to_local<P: AsRef<Path> + Send, R: RemoteFS>(
    client_id: &str,
    remote_fs: R,
    local_path: P,
  ) -> impl std::future::Future<Output = SyncResult<()>> + Send;
}
