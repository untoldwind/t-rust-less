#[cfg(feature = "pcloud")]
use crate::cli::PcloudRegion;
use crate::cli::{Args, Remote};
use anyhow::Result;

pub async fn initialize(args: &Args) -> Result<()> {
  match args.remote {
    #[cfg(feature = "dropbox")]
    Remote::Dropbox => initialize_dropbox(),
    #[cfg(feature = "pcloud")]
    Remote::Pcloud => initialize_pcloud(args.pcloud_region).await,
  }
}

#[cfg(feature = "dropbox")]
fn initialize_dropbox() -> Result<()> {
  let initializer = t_rust_less_synctool::dropbox::initialize::DropboxInitializer::new()?;
  println!("Authenticate via browser: {}", initializer.auth_url);

  open::that(initializer.auth_url.as_str())?;

  let token = initializer.wait_for_authentication()?;

  println!("Token: {}", token.as_str());

  Ok(())
}

#[cfg(feature = "pcloud")]
async fn initialize_pcloud(pcloud_region: PcloudRegion) -> Result<()> {
  use dialoguer::{Input, Password};
  use pcloud::Region;
  use t_rust_less_synctool::pcloud::initialize::get_pcloud_token;

  let username = Input::<String>::new().with_prompt("Username").interact_text()?;
  let password = Password::new().with_prompt("Password").interact()?;
  let region = match pcloud_region {
    PcloudRegion::Eu => Region::Eu,
    PcloudRegion::Us => Region::Us,
  };
  let token = get_pcloud_token(region, username.into(), password.into()).await?;

  println!("Token: {}", token.as_str());

  Ok(())
}
