use anyhow::Result;
use t_rust_less_synctool::layout::SyncProgress;
#[cfg(feature = "dropbox")]
use tokio::sync::mpsc::Sender;

use crate::cli::Args;
#[cfg(feature = "pcloud")]
use crate::cli::PcloudRegion;
#[cfg(feature = "dropbox")]
use crate::cli::Remote;

pub async fn sync(args: &Args) -> Result<()> {
  let (tx, mut rx) = tokio::sync::mpsc::channel::<SyncProgress>(10);

  tokio::spawn(async move {
    while let Some(progress) = rx.recv().await {
      println!("{}/{}: {}", progress.step, progress.remaining, progress.message);
    }
  });

  match args.remote {
    #[cfg(feature = "dropbox")]
    Remote::Dropbox => sync_dropbox(tx).await,
    #[cfg(feature = "pcloud")]
    Remote::Pcloud => sync_pcloud(args.pcloud_region, tx).await,
  }
}

#[cfg(feature = "dropbox")]
async fn sync_dropbox(_sync_progress: Sender<SyncProgress>) -> Result<()> {
  todo!()
}

#[cfg(feature = "pcloud")]
async fn sync_pcloud(_pcloud_region: PcloudRegion, _sync_progress: Sender<SyncProgress>) -> Result<()> {
  todo!()
}
