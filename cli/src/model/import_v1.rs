use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use std::collections::btree_map::BTreeMap;
use t_rust_less_lib::api::{SecretAttachment, SecretType};
use t_rust_less_lib::memguard::weak::ZeroingString;

#[derive(Clone, Serialize, Deserialize)]
pub struct SecretV1 {
  pub id: String,
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub versions: Vec<SecretVersionV1>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SecretVersionV1 {
  pub timestamp: DateTime<Utc>,
  pub name: ZeroingString,
  pub tags: Option<Vec<ZeroingString>>,
  pub urls: Option<Vec<ZeroingString>>,
  pub properties: BTreeMap<String, ZeroingString>,
  pub attachments: Option<Vec<SecretAttachment>>,
  pub deleted: bool,
}
