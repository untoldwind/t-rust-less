use std::{future::Future, time::SystemTime};

use futures::{stream, StreamExt};
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::SyncResult;

pub struct RemoteFileMetadata {
  pub name: String,
  pub path: String,
  pub size: u64,
  pub mtime: SystemTime,
}

pub struct DownloadTask<'a, W> {
  pub path: &'a str,
  pub target: &'a mut W,
}

pub struct DownloadTaskResult {
  pub path: String,
  pub result: SyncResult<u64>,
}

pub struct UploadTask<'a, R> {
  pub path: &'a str,
  pub size: u64,
  pub source: &'a mut R,
}

pub struct UploadTaskResult {
  pub path: String,
  pub result: SyncResult<u64>,
}

pub trait RemoteFS: Send + Sync {
  fn list_folder(&self, path: &str) -> impl Future<Output = SyncResult<Vec<RemoteFileMetadata>>> + Send;

  fn ensure_folders(&self, paths: &[&str]) -> impl Future<Output = SyncResult<()>> + Send;

  fn download_to<W: AsyncWrite + Send + Unpin>(
    &self,
    task: &mut DownloadTask<'_, W>,
  ) -> impl Future<Output = SyncResult<u64>> + Send;

  fn parallel_download_to<W: AsyncWrite + Send + Unpin>(
    &self,
    parallel: usize,
    tasks: &mut [DownloadTask<'_, W>],
  ) -> impl Future<Output = Vec<DownloadTaskResult>> + Send {
    stream::iter(tasks)
      .map(async |task| {
        let result = self.download_to(task).await;
        DownloadTaskResult {
          path: task.path.to_string(),
          result,
        }
      })
      .buffer_unordered(parallel)
      .collect()
  }

  fn upload_from<R: AsyncRead + Send>(
    &self,
    task: &mut UploadTask<'_, R>,
  ) -> impl Future<Output = SyncResult<u64>> + Send;

  fn parallel_upload_from<R: AsyncRead + Send>(
    &self,
    parallel: usize,
    tasks: &mut [UploadTask<'_, R>],
  ) -> impl Future<Output = Vec<UploadTaskResult>> + Send {
    stream::iter(tasks)
      .map(async |task| {
        let result = self.upload_from(task).await;
        UploadTaskResult {
          path: task.path.to_string(),
          result,
        }
      })
      .buffer_unordered(parallel)
      .collect()
  }
}
