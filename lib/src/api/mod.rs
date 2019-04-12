use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
  initialized: bool,
  locked: bool,
  autolock_at: Option<DateTime<Utc>>,
  version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
  name: String,
  email: String,
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
  id: String,
  name: String,
  #[serde(rename = "nameHighlights")]
  name_highlights: Vec<u32>,
  #[serde(rename = "type")]
  secret_type: SecretType,
  taps: Vec<String>,
  urls: Vec<String>,
  timestamp: DateTime<Utc>,
  deleted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretList {
  all_tags: Vec<String>,
  entries: Vec<SecretEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretAttachment {
  name: String,
  mime_type: String,
  content: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretVersion {
  timestamp: DateTime<Utc>,
  name: String,
  tags: Vec<String>,
  urls: Vec<String>,
  properties: BTreeMap<String, String>,
  attachments: Vec<SecretAttachment>,
  deleted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordEstimate {
  password: String,
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
  id: String,
  #[serde(rename = "type")]
  secret_type: SecretType,
  current: SecretVersion,
  versions: Vec<SecretVersion>,
  password_strengths: HashMap<String, PasswordStrength>,
}
