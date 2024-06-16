use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[cfg_attr(feature = "with_specta", derive(specta::Type))]
#[zeroize(drop)]
pub struct StoreConfig {
  pub name: String,
  pub store_url: String,
  pub remote_url: Option<String>,
  #[serde(default)]
  pub sync_interval_sec: u32,
  pub client_id: String,
  pub autolock_timeout_secs: u64,
  pub default_identity_id: Option<String>,
}
