use futures::stream::TryStreamExt;
use pcloud::{stream::StreamingLink, Client, Credentials, Region};
use tokio::io::{self, AsyncRead, AsyncWrite};

use crate::{
  error::{SyncError, SyncResult},
  remote_fs::{RemoteFS, RemoteFileMetadata},
};

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
    todo!()
  }

  async fn ensure_folders(&self, paths: &[&str]) -> SyncResult<()> {
    todo!()
  }

  async fn download_to<W: AsyncWrite + Unpin>(&self, path: &str, target: &mut W) -> SyncResult<u64> {
    let file_links = self.client.get_file_link(format!("{}{}", self.base_dir, path)).await?;

    for link in file_links.links() {
      match self.check_file_link(link).await {
        Ok(success) => {
          let stream = success.bytes_stream().map_err(std::io::Error::other);
          let mut source = tokio_util::io::StreamReader::new(stream);

          io::copy(&mut source, target).await?;
        }
        Err(err) => log::warn!("PCloud link failed: {err}"),
      };
    }
    Err(SyncError::Generic("Download failed: No more links to try".to_string()))
  }

  async fn upload_from<R: AsyncRead>(&self, path: &str, source: &mut R) -> SyncResult<u64> {
    todo!()
  }
}
