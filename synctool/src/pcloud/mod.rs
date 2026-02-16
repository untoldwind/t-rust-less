use futures::stream::TryStreamExt;
use pcloud::{entry::Entry, folder::FolderIdentifier, stream::StreamingLink, Client, Credentials, Region};
use tokio::io::{self, AsyncRead, AsyncWrite};

use crate::{
  error::{SyncError, SyncResult},
  remote_fs::{DownloadTask, RemoteFS, RemoteFileMetadata, UploadTask},
};

pub mod initialize;

pub struct PCloudRemoteFS {
  client: Client,
  base_dir: String,
}

impl PCloudRemoteFS {
  pub fn new(token: &str, region: Region, base_dir: &str) -> SyncResult<Self> {
    let reqwest_builder = pcloud::reqwest::Client::builder().user_agent("t-rust-less-synctool");

    let mut builder = Client::builder();
    builder.set_credentials(Credentials::AccessToken {
      access_token: token.to_string(),
    });
    builder.set_region(region);
    builder.set_client_builder(reqwest_builder);
    let client = builder.build()?;

    Ok(PCloudRemoteFS {
      client,
      base_dir: base_dir.to_string(),
    })
  }

  async fn check_file_link(&self, link: StreamingLink<'_>) -> SyncResult<pcloud::reqwest::Response> {
    let res = pcloud::reqwest::get(link.to_string()).await?;
    res.error_for_status_ref()?;

    Ok(res)
  }
}

impl RemoteFS for PCloudRemoteFS {
  async fn list_folder(&self, path: &str) -> SyncResult<Vec<RemoteFileMetadata>> {
    let folder = self
      .client
      .list_folder(FolderIdentifier::Path(format!("{}/{}", self.base_dir, path).into()))
      .await?;
    if let Some(entries) = folder.contents {
      Ok(
        entries
          .into_iter()
          .filter_map(|e| match e {
            Entry::File(file) => Some(RemoteFileMetadata {
              path: format!("{}/{}/{}", self.base_dir, path, file.base.name),
              name: file.base.name,
              size: file.size.unwrap_or_default() as u64,
              mtime: file.base.modified.into(),
            }),
            _ => None,
          })
          .collect(),
      )
    } else {
      Ok(vec![])
    }
  }

  async fn ensure_folders(&self, paths: &[&str]) -> SyncResult<()> {
    todo!()
  }

  async fn download_to<W: AsyncWrite + Unpin>(&self, task: &mut DownloadTask<'_, W>) -> SyncResult<u64> {
    let file_links = self
      .client
      .get_file_link(format!("{}{}", self.base_dir, task.path))
      .await?;

    for link in file_links.links() {
      match self.check_file_link(link).await {
        Ok(success) => {
          let stream = success.bytes_stream().map_err(std::io::Error::other);
          let mut source = tokio_util::io::StreamReader::new(stream);

          io::copy(&mut source, task.target).await?;
        }
        Err(err) => log::warn!("PCloud link failed: {err}"),
      };
    }
    Err(SyncError::Generic("Download failed: No more links to try".to_string()))
  }

  async fn upload_from<R: AsyncRead>(&self, task: &mut UploadTask<'_, R>) -> SyncResult<u64> {
    todo!()
  }
}
