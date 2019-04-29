use crate::api_capnp::store_config;
use crate::service::ServiceResult;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreConfig {
  pub name: String,
  pub store_url: String,
  pub client_id: String,
  pub autolock_timeout_secs: u64,
}

impl StoreConfig {
  pub fn from_reader(reader: store_config::Reader) -> capnp::Result<StoreConfig> {
    Ok(StoreConfig {
      name: reader.get_name()?.to_string(),
      store_url: reader.get_store_url()?.to_string(),
      client_id: reader.get_client_id()?.to_string(),
      autolock_timeout_secs: reader.get_autolock_timeout_secs(),
    })
  }

  pub fn to_builder(&self, mut builder: store_config::Builder) {
    builder.set_name(&self.name);
    builder.set_store_url(&self.store_url);
    builder.set_client_id(&self.client_id);
    builder.set_autolock_timeout_secs(self.autolock_timeout_secs);
  }
}

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
