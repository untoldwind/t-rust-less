use crate::error::{exit_with_error, ExtResult};
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
  pub store_url: String,
  pub client_id: String,
  pub autolock_timeout: Duration,
}

pub fn default_store_dir() -> PathBuf {
  let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

  dirs::document_dir().unwrap_or(home_dir).join("t-rust-less-store")
}

pub fn default_autolock_timeout() -> Duration {
  Duration::from_secs(300)
}

pub fn config_file() -> PathBuf {
  let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
  dirs::config_dir()
    .unwrap_or_else(|| home_dir.join(".t-rust-less"))
    .join("t-rust-less.toml")
}

pub fn read_config() -> Option<Config> {
  let config_file = config_file();
  match File::open(&config_file) {
    Ok(mut index_file) => {
      let mut content = vec![];

      index_file
        .read_to_end(&mut content)
        .ok_or_exit(&format!("Unable to read '{}': ", config_file.to_string_lossy()));

      Some(toml::from_slice::<Config>(&content).ok_or_exit("Incalid config file: "))
    }
    Err(ref err) if err.kind() == io::ErrorKind::NotFound => None,
    Err(err) => {
      exit_with_error(&format!("Unable to open '{}': ", config_file.to_string_lossy()), err);
      unreachable!()
    }
  }
}

pub fn write_config(config: &Config) -> io::Result<()> {
  let content = toml::to_string_pretty(config).unwrap();
  let config_file = config_file();

  fs::create_dir_all(&config_file.parent().unwrap())?;

  let mut file = File::create(&config_file)?;

  file.write_all(content.as_bytes())?;

  Ok(())
}
