use crate::service::ServiceResult;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreConfig {
  pub name: String,
  pub store_url: String,
  pub client_id: String,
  pub autolock_timeout: Duration,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
  pub stores: HashMap<String, StoreConfig>,
  pub default_store: Option<String>,
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

pub fn read_config() -> ServiceResult<Option<Config>> {
  let config_file = config_file();
  match File::open(&config_file) {
    Ok(mut index_file) => {
      let mut content = vec![];

      index_file.read_to_end(&mut content)?;

      Ok(Some(toml::from_slice::<Config>(&content)?))
    }
    Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(err) => Err(err.into()),
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
