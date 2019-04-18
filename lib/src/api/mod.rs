use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::memguard::weak::{ZeroingBytes, ZeroingString};
use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

/// Status information of a secrets store
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
  pub locked: bool,
  pub unlocked_by: Option<Identity>,
  pub autolock_at: Option<DateTime<Utc>>,
  pub version: String,
}

/// An Identity that might be able to unlock a
/// secrets store and be a recipient of secrets.
///
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Identity {
  pub id: String,
  pub name: String,
  pub email: String,
}

/// General type of a secret.
///
/// This only serves as a hint for an UI.
///
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecretType {
  Login,
  Note,
  Licence,
  Wlan,
  Password,
  Other,
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
  pub secret_id: ZeroingString,
  pub secret_type: SecretType,
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
  #[serde(default)]
  pub recipients: Vec<ZeroingString>,
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
