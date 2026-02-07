use thiserror::Error;

#[derive(Debug, Error)]
pub enum SyncError {
  #[error("Generic: {0}")]
  Generic(String),
  #[error("IO")]
  IO(#[from] std::io::Error),
  #[error("URL")]
  URL(#[from] url::ParseError),
  #[cfg(feature = "dropbox")]
  #[error("dropbox error")]
  Dropbox(#[from] dropbox_sdk::Error),
  #[cfg(feature = "dropbox")]
  #[error("dropbox download error")]
  DropboxDownload(#[from] dropbox_sdk::Error<dropbox_sdk::files::DownloadError>),
  #[cfg(feature = "dropbox")]
  #[error("dropbox list folder error")]
  DropboxListFolder(#[from] dropbox_sdk::Error<dropbox_sdk::files::ListFolderError>),
  #[cfg(feature = "dropbox")]
  #[error("dropbox list folder continue error")]
  DropboxListFolderContinue(#[from] dropbox_sdk::Error<dropbox_sdk::files::ListFolderContinueError>),
  #[cfg(feature = "pcloud")]
  #[error("pcloud error")]
  PCloud(#[from] pcloud::Error),
  #[cfg(feature = "pcloud")]
  #[error("pcloud error")]
  PCloudBuilder(#[from] pcloud::builder::Error),
  #[cfg(feature = "pcloud")]
  #[error("pcloud error")]
  PCloudReqwest(#[from] pcloud::reqwest::Error),
}

pub type SyncResult<T> = Result<T, SyncError>;
