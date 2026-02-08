use crate::cli::Remote;
use anyhow::Result;

pub fn initialize(remote: Remote) -> Result<()> {
  match remote {
    #[cfg(feature = "dropbox")]
    Remote::Dropbox => initialize_dropbox(),
    #[cfg(feature = "pcloud")]
    Remote::PCloud => initialize_pcloud(),
  }
}

#[cfg(feature = "dropbox")]
fn initialize_dropbox() -> Result<()> {
  let initializer = t_rust_less_synctool::dropbox::initialize::DropboxInitializer::new()?;
  println!("Authenticate via browser: {}", initializer.auth_url);

  open::that(initializer.auth_url.as_str())?;

  let token = initializer.wait_for_authentication()?;

  println!("Token: {}", token);
  
  Ok(())
}

#[cfg(feature = "pcloud")]
fn initialize_pcloud() -> Result<()> {
  todo!()
}
