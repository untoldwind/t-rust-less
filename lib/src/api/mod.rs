use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::memguard::weak::{ZeroingBytes, ZeroingString};
use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
  pub locked: bool,
  pub autolock_at: Option<DateTime<Utc>>,
  pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
  pub id: String,
  pub name: String,
  pub email: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretType {
  Login,
  Note,
  Licence,
  Wlan,
  Password,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretListFilter {
  url: Option<String>,
  tag: Option<String>,
  #[serde(rename = "type")]
  secret_type: Option<SecretType>,
  name: Option<String>,
  deleted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretEntry {
  id: ZeroingString,
  name: ZeroingString,
  #[serde(rename = "nameHighlights")]
  name_highlights: Vec<u32>,
  #[serde(rename = "type")]
  secret_type: SecretType,
  taps: Vec<ZeroingString>,
  urls: Vec<ZeroingString>,
  timestamp: DateTime<Utc>,
  deleted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretList {
  all_tags: Vec<ZeroingString>,
  entries: Vec<SecretEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretAttachment {
  name: String,
  mime_type: String,
  content: ZeroingBytes,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretVersion {
  timestamp: DateTime<Utc>,
  name: ZeroingString,
  tags: Vec<ZeroingString>,
  urls: Vec<ZeroingString>,
  properties: BTreeMap<String, ZeroingString>,
  attachments: Vec<SecretAttachment>,
  deleted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordEstimate {
  password: ZeroingString,
  inputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordStrength {
  entropy: f64,
  crack_time: f64,
  #[serde(rename = "crackTimeDisplay")]
  crack_time_display: String,
  score: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Secret {
  id: ZeroingString,
  #[serde(rename = "type")]
  secret_type: SecretType,
  current: SecretVersion,
  versions: Vec<SecretVersion>,
  password_strengths: HashMap<String, PasswordStrength>,
}
