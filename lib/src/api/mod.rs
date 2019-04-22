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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SecretType {
  Login,
  Note,
  Licence,
  Wlan,
  Password,
  Other,
}

/// A combination of filter criterias to search for a secret.
///
/// All criterias are supposed to be combined by AND (i.e. all criterias have
/// to match).
/// Match on `name` is supposed to be "fuzzy" by some fancy scheme.
///
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct SecretListFilter {
  pub url: Option<String>,
  pub tag: Option<String>,
  #[serde(rename = "type")]
  pub secret_type: Option<SecretType>,
  pub name: Option<String>,
  pub deleted: bool,
}

/// SecretEntry contains all the information of a secrets that should be
/// indexed.
///
/// Even though a SecretEntry does no contain a password it is still supposed to
/// be sensitive data.
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretEntry {
  pub id: String,
  pub name: ZeroingString,
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub tags: Vec<ZeroingString>,
  pub urls: Vec<ZeroingString>,
  pub timestamp: DateTime<Utc>,
  pub deleted: bool,
}

/// Representation of a filter match to a SecretEntry.
///
/// For the most part this is just the entry itself with some additional information
/// which parts should be highlighted in the UI
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretEntryMatch {
  pub entry: SecretEntry,
  /// Matching score of the name
  pub name_score: isize,
  /// Array of positions (single chars) to highlight in the name of the entry
  pub name_highlights: Vec<usize>,
  /// Array of matching urls
  pub url_highlights: Vec<usize>,
  /// Array of matching tags
  pub tags_highlights: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretList {
  pub all_tags: Vec<ZeroingString>,
  pub entries: Vec<SecretEntryMatch>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretAttachment {
  name: String,
  mime_type: String,
  content: ZeroingBytes,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretVersion {
  pub secret_id: String,
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
