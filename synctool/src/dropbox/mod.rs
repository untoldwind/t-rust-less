mod initialize;

use std::collections::VecDeque;

pub use initialize::*;

use dropbox_sdk::{
  async_client_trait::UserAuthClient,
  async_routes::files::{self, ListFolderError},
  default_async_client::UserAuthDefaultClient,
  oauth2::Authorization,
};
use tokio::io::{self, AsyncWrite};
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::{
  error::{SyncError, SyncResult},
  remote_fs::RemoteFS,
};

pub const APP_KEY: &str = "3q0sff542l6r3ly";

pub struct DroboxRemoteFS {
  client: UserAuthDefaultClient,
  base_dir: String,
}

impl DroboxRemoteFS {
  pub fn new(token: &str, base_dir: &str) -> SyncResult<DroboxRemoteFS> {
    let authorization = Authorization::load(APP_KEY.to_string(), token)
      .ok_or_else(|| SyncError::Generic("Invalid dropbox token".to_string()))?;
    let client = UserAuthDefaultClient::new(authorization);

    Ok(DroboxRemoteFS {
      client,
      base_dir: base_dir.to_string(),
    })
  }
}

impl RemoteFS for DroboxRemoteFS {
  async fn download_to<W: AsyncWrite + Unpin>(&self, path: String, target: &mut W) -> SyncResult<u64> {
    let result = files::download(&self.client, &files::DownloadArg::new(path), None, None).await?;
    let content = result.body.ok_or_else(|| SyncError::Generic("No body".to_string()))?;
    let bytes = io::copy(&mut content.compat(), target).await?;

    Ok(bytes)
  }
}

impl std::fmt::Debug for DroboxRemoteFS {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DropboxBlockStore")
      .field("base_dir", &self.base_dir)
      .finish()
  }
}

/*
fn list_directory<T: UserAuthClient>(
  client: &T,
  path: String,
  recursive: bool,
) -> SyncResult<DirectoryIterator<'_, T>> {
  let requested_path = if path == "/" { String::new() } else { path };
  let result = match files::list_folder(
    client,
    &files::ListFolderArg::new(requested_path).with_recursive(recursive),
  ) {
    Ok(result) => result,
    Err(dropbox_sdk::Error::Api(ListFolderError::Path(_))) => {
      return Ok(DirectoryIterator {
        client,
        cursor: None,
        buffer: VecDeque::new(),
      })
    }
    Err(err) => return Err(err.into()),
  };

  let cursor = if result.has_more { Some(result.cursor) } else { None };

  Ok(DirectoryIterator {
    client,
    cursor,
    buffer: result.entries.into(),
  })
}

struct DirectoryIterator<'a, T: UserAuthClient> {
  client: &'a T,
  buffer: VecDeque<files::Metadata>,
  cursor: Option<String>,
}

impl<T: UserAuthClient> Iterator for DirectoryIterator<'_, T> {
  type Item = SyncResult<files::Metadata>;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(entry) = self.buffer.pop_front() {
      Some(Ok(entry))
    } else if let Some(cursor) = self.cursor.take() {
      match files::list_folder_continue(self.client, &files::ListFolderContinueArg::new(cursor)) {
        Ok(result) => {
          self.buffer.extend(result.entries);
          if result.has_more {
            self.cursor = Some(result.cursor);
          }
          self.buffer.pop_front().map(Ok)
        }
        Err(e) => Some(Err(e.into())),
      }
    } else {
      None
    }
  }
}

*/
