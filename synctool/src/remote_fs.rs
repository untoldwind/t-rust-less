use std::{future::Future, time::SystemTime};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::SyncResult;

pub struct RemoteFileMetadata {
  pub path: String,
  pub size: u64,
  pub mtime: SystemTime,
}

pub trait RemoteFS {
  fn list_folder(&self, path: &str) -> impl Future<Output = SyncResult<Vec<RemoteFileMetadata>>>;

  fn ensure_folders(&self, paths: &[&str]) -> impl Future<Output = SyncResult<()>>;

  fn download_to<W: AsyncWrite + Unpin>(&self, path: &str, target: &mut W) -> impl Future<Output = SyncResult<u64>>;

  fn upload_from<R: AsyncRead>(&self, path: &str, source: &mut R) -> impl Future<Output = SyncResult<u64>>;
}
