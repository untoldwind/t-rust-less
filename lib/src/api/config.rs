use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct StoreConfig {
  pub name: String,
  pub store_url: String,
  pub client_id: String,
  pub autolock_timeout_secs: u64,
  pub default_identity_id: Option<String>,
}
