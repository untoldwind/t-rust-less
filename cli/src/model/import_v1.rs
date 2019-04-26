use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use std::collections::btree_map::BTreeMap;
use t_rust_less_lib::api::{SecretAttachment, SecretType};
use t_rust_less_lib::memguard::weak::ZeroingString;

#[derive(Clone, Serialize, Deserialize)]
pub struct SecretV1 {
  id: String,
  #[serde(rename = "type")]
  secret_type: SecretType,
  versions: Vec<SecretVersionV1>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SecretVersionV1 {
  pub timestamp: DateTime<Utc>,
  pub name: ZeroingString,
  #[serde(default)]
  pub tags: Vec<ZeroingString>,
  #[serde(default)]
  pub urls: Vec<ZeroingString>,
  pub properties: BTreeMap<String, ZeroingString>,
  #[serde(default)]
  pub attachments: Vec<SecretAttachment>,
  #[serde(default)]
  pub deleted: bool,
}
