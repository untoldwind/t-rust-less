use std::path::PathBuf;
use std::time::Duration;

pub fn default_store_dir(store_name: &str) -> PathBuf {
  let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

  dirs::document_dir().unwrap_or(home_dir).join(store_name)
}

pub fn default_autolock_timeout() -> Duration {
  Duration::from_secs(300)
}
