use std::path::Path;

use crate::{error::SyncResult, layout::Layout, remote_fs::RemoteFS};

pub struct LocalDirLayout();

impl Layout for LocalDirLayout {
  async fn sync_remote_to_local<P: AsRef<Path> + Send, R: RemoteFS>(
    client_id: &str,
    remote_fs: R,
    local_path: P,
  ) -> SyncResult<()> {
    todo!()
  }
}
