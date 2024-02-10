use crate::api::StoreConfig;
use crate::service::ServiceResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
  pub default_store: Option<String>,
  pub stores: HashMap<String, StoreConfig>,
}

pub fn config_file() -> PathBuf {
  let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
  dirs::config_dir()
    .map(|configs| configs.join("t-rust-less"))
    .unwrap_or_else(|| home_dir.join(".t-rust-less"))
    .join("config.toml")
}

pub fn read_config() -> ServiceResult<Option<Config>> {
  let config_file = config_file();
  match File::open(config_file) {
    Ok(mut index_file) => {
      let mut content = String::new();

      index_file.read_to_string(&mut content)?;

      Ok(Some(toml::from_str::<Config>(&content)?))
    }
    Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(err) => Err(err.into()),
  }
}

pub fn write_config(config: &Config) -> io::Result<()> {
  let content = toml::to_string_pretty(config).unwrap();
  let config_file = config_file();

  fs::create_dir_all(config_file.parent().unwrap())?;

  let mut file = File::create(&config_file)?;

  file.write_all(content.as_bytes())?;

  Ok(())
}
