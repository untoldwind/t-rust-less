use crate::api::StoreConfig;
use crate::service::ServiceResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
  fs::{self, File},
  io::{self, Read, Write},
  path::PathBuf,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Config {
  pub default_store: Option<String>,
  pub stores: HashMap<String, StoreConfig>,
}

pub trait ConfigProvider: Send + Sync {
  fn read_config(&self) -> ServiceResult<Option<Config>>;

  fn write_config(&self, config: &Config) -> ServiceResult<()>;
}

pub struct LocalConfigProvider {
  config_file: PathBuf,
}

impl LocalConfigProvider {
  pub fn config_file() -> PathBuf {
    let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    dirs::config_dir()
      .map(|configs| configs.join("t-rust-less"))
      .unwrap_or_else(|| home_dir.join(".t-rust-less"))
      .join("config.toml")
  }
}

impl Default for LocalConfigProvider {
  fn default() -> Self {
    LocalConfigProvider {
      config_file: Self::config_file(),
    }
  }
}

impl ConfigProvider for LocalConfigProvider {
  fn read_config(&self) -> ServiceResult<Option<Config>> {
    match File::open(&self.config_file) {
      Ok(mut index_file) => {
        let mut content = String::new();

        index_file.read_to_string(&mut content)?;

        Ok(Some(toml::from_str::<Config>(&content)?))
      }
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
      Err(err) => Err(err.into()),
    }
  }

  fn write_config(&self, config: &Config) -> ServiceResult<()> {
    let content = toml::to_string_pretty(config).unwrap();

    fs::create_dir_all(self.config_file.parent().unwrap())?;

    let mut file = File::create(&self.config_file)?;

    file.write_all(content.as_bytes())?;

    Ok(())
  }
}
