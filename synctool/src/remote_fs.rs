use tokio::io::AsyncWrite;

use crate::error::SyncResult;

pub trait RemoteFS {
  fn download_to<W: AsyncWrite + Unpin>(
    &self,
    path: String,
    target: &mut W,
  ) -> impl std::future::Future<Output = SyncResult<u64>>;
}
