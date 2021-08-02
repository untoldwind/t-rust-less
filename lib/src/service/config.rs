use crate::service::ServiceResult;
use crate::{api::read_option, api_capnp::store_config};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use zeroize::Zeroize;

#[derive(Clone, Debug, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct StoreConfig {
  pub name: String,
  pub store_url: String,
  pub client_id: String,
  pub autolock_timeout_secs: u64,
  pub default_identity_id: Option<String>,
}

impl StoreConfig {
  pub fn from_reader(reader: store_config::Reader) -> capnp::Result<StoreConfig> {
    Ok(StoreConfig {
      name: reader.get_name()?.to_string(),
      store_url: reader.get_store_url()?.to_string(),
      client_id: reader.get_client_id()?.to_string(),
      autolock_timeout_secs: reader.get_autolock_timeout_secs(),
      default_identity_id: read_option(reader.get_default_identity_id()?)?.map(ToString::to_string),
    })
  }

  pub fn to_builder(&self, mut builder: store_config::Builder) -> capnp::Result<()> {
    builder.set_name(&self.name);
    builder.set_store_url(&self.store_url);
    builder.set_client_id(&self.client_id);
    builder.set_autolock_timeout_secs(self.autolock_timeout_secs);
    match &self.default_identity_id {
      Some(default_identity_id) => builder
        .reborrow()
        .init_default_identity_id()
        .set_some(capnp::text::new_reader(default_identity_id.as_bytes())?)?,
      None => builder.reborrow().init_default_identity_id().set_none(()),
    }

    Ok(())
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
