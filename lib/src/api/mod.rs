use crate::secrets_store_capnp::{self, secret_entry, secret_version_ref};
use capnp::text_list;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;
use zeroize::Zeroize;
mod command;
mod config;
mod event;
mod zeroize_datetime;

#[cfg(test)]
mod tests;

pub use command::*;
pub use config::*;
pub use event::*;
pub use zeroize_datetime::*;

pub const PROPERTY_USERNAME: &str = "username";
pub const PROPERTY_PASSWORD: &str = "password";
pub const PROPERTY_TOTP_URL: &str = "totpUrl";
pub const PROPERTY_NOTES: &str = "notes";

/// Status information of a secrets store
///
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct Status {
  pub locked: bool,
  pub unlocked_by: Option<Identity>,
  pub autolock_at: Option<ZeroizeDateTime>,
  pub version: String,
  pub autolock_timeout: u64,
}

/// An Identity that might be able to unlock a
/// secrets store and be a recipient of secrets.
///
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct Identity {
  pub id: String,
  pub name: String,
  pub email: String,
  pub hidden: bool,
}

impl std::fmt::Display for Identity {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{} <{}>", self.name, self.email)
  }
}

/// General type of a secret.
///
/// This only serves as a hint for an UI.
///
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SecretType {
  Login,
  Note,
  Licence,
  Wlan,
  Password,
  #[serde(other)]
  Other,
}

impl Zeroize for SecretType {
  fn zeroize(&mut self) {
    *self = SecretType::Other
  }
}

impl SecretType {
  /// Get the commonly used property name that may contain a password.
  ///
  /// The values of these properties are automatically estimated for the strengths.
  pub fn password_properties(&self) -> &[&str] {
    match self {
      SecretType::Login => &[PROPERTY_PASSWORD],
      SecretType::Note => &[],
      SecretType::Licence => &[],
      SecretType::Wlan => &[PROPERTY_PASSWORD],
      SecretType::Password => &[PROPERTY_PASSWORD],
      SecretType::Other => &[],
    }
  }

  pub fn from_reader(api: secrets_store_capnp::SecretType) -> Self {
    match api {
      secrets_store_capnp::SecretType::Login => SecretType::Login,
      secrets_store_capnp::SecretType::Licence => SecretType::Licence,
      secrets_store_capnp::SecretType::Wlan => SecretType::Wlan,
      secrets_store_capnp::SecretType::Note => SecretType::Note,
      secrets_store_capnp::SecretType::Password => SecretType::Password,
      secrets_store_capnp::SecretType::Other => SecretType::Other,
    }
  }

  pub fn to_builder(self) -> secrets_store_capnp::SecretType {
    match self {
      SecretType::Login => secrets_store_capnp::SecretType::Login,
      SecretType::Licence => secrets_store_capnp::SecretType::Licence,
      SecretType::Note => secrets_store_capnp::SecretType::Note,
      SecretType::Wlan => secrets_store_capnp::SecretType::Wlan,
      SecretType::Password => secrets_store_capnp::SecretType::Password,
      SecretType::Other => secrets_store_capnp::SecretType::Other,
    }
  }
}

impl fmt::Display for SecretType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      SecretType::Login => write!(f, "Login"),
      SecretType::Note => write!(f, "Note"),
      SecretType::Licence => write!(f, "Licence"),
      SecretType::Wlan => write!(f, "WLAN"),
      SecretType::Password => write!(f, "Password"),
      SecretType::Other => write!(f, "Other"),
    }
  }
}

/// A combination of filter criterias to search for a secret.
///
/// All criterias are supposed to be combined by AND (i.e. all criterias have
/// to match).
/// Match on `name` is supposed to be "fuzzy" by some fancy scheme.
///
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretListFilter {
  pub url: Option<String>,
  pub tag: Option<String>,
  #[serde(rename = "type")]
  pub secret_type: Option<SecretType>,
  pub name: Option<String>,
  #[serde(default)]
  pub deleted: bool,
}

/// SecretEntry contains all the information of a secrets that should be
/// indexed.
///
/// Even though a SecretEntry does no contain a password it is still supposed to
/// be sensitive data.
///
/// See SecretVersion for further detail.
///
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretEntry {
  pub id: String,
  pub name: String,
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub tags: Vec<String>,
  pub urls: Vec<String>,
  pub timestamp: ZeroizeDateTime,
  pub deleted: bool,
}

impl SecretEntry {
  pub fn from_reader(reader: secret_entry::Reader) -> capnp::Result<Self> {
    Ok(SecretEntry {
      id: reader.get_id()?.to_string(),
      timestamp: Utc.timestamp_millis(reader.get_timestamp()).into(),
      name: reader.get_name()?.to_string(),
      secret_type: SecretType::from_reader(reader.get_type()?),
      tags: reader
        .get_tags()?
        .into_iter()
        .map(|t| t.map(|t| t.to_string()))
        .collect::<capnp::Result<Vec<String>>>()?,
      urls: reader
        .get_urls()?
        .into_iter()
        .map(|u| u.map(|u| u.to_string()))
        .collect::<capnp::Result<Vec<String>>>()?,
      deleted: reader.get_deleted(),
    })
  }

  pub fn to_builder(&self, mut builder: secret_entry::Builder) {
    builder.set_id(&self.id);
    builder.set_timestamp(self.timestamp.timestamp_millis());
    builder.set_name(&self.name);
    builder.set_type(self.secret_type.to_builder());
    let mut tags = builder.reborrow().init_tags(self.tags.len() as u32);
    for (idx, tag) in self.tags.iter().enumerate() {
      tags.set(idx as u32, tag)
    }
    let mut urls = builder.reborrow().init_urls(self.urls.len() as u32);
    for (idx, url) in self.urls.iter().enumerate() {
      urls.set(idx as u32, url)
    }
    builder.set_deleted(self.deleted);
  }
}

impl Ord for SecretEntry {
  fn cmp(&self, other: &Self) -> Ordering {
    match self.name.cmp(&other.name) {
      Ordering::Equal => self.id.cmp(&other.id),
      ord => ord,
    }
  }
}

impl PartialOrd for SecretEntry {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for SecretEntry {
  fn eq(&self, other: &Self) -> bool {
    self.id.eq(&other.id)
  }
}

/// Representation of a filter match to a SecretEntry.
///
/// For the most part this is just the entry itself with some additional information
/// which parts should be highlighted in the UI
///
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Zeroize)]
#[zeroize(drop)]
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

impl Ord for SecretEntryMatch {
  fn cmp(&self, other: &Self) -> Ordering {
    match other.name_score.cmp(&self.name_score) {
      Ordering::Equal => self.entry.cmp(&other.entry),
      ord => ord,
    }
  }
}

impl PartialOrd for SecretEntryMatch {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for SecretEntryMatch {
  fn eq(&self, other: &Self) -> bool {
    self.entry.eq(&other.entry)
  }
}

/// Convenient wrapper of a list of SecretEntryMatch'es.
///
/// Also contains a unique list of tags of all secrets (e.g. to support autocompletion)
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretList {
  pub all_tags: Vec<String>,
  pub entries: Vec<SecretEntryMatch>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SecretProperties(BTreeMap<String, String>);

impl SecretProperties {
  pub fn new(properties: BTreeMap<String, String>) -> Self {
    SecretProperties(properties)
  }

  pub fn has_non_empty(&self, name: &str) -> bool {
    matches!(self.0.get(name), Some(value) if !value.is_empty())
  }

  pub fn get(&self, name: &str) -> Option<&String> {
    self.0.get(name)
  }

  pub fn len(&self) -> usize {
    self.0.len()
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
    self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
  }
}

impl Drop for SecretProperties {
  fn drop(&mut self) {
    self.zeroize()
  }
}

impl Zeroize for SecretProperties {
  fn zeroize(&mut self) {
    self.0.values_mut().for_each(Zeroize::zeroize);
  }
}

/// Some short of attachment to a secret.
///
/// Be aware that t-rust-less is supposed to be a password store, do not misuse it as a
/// secure document store. Nevertheless, sometimes it might be convenient added some
/// sort of (small) document to a password.
///
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretAttachment {
  name: String,
  mime_type: String,
  content: Vec<u8>,
}

/// SecretVersion holds all information of a specific version of a secret.
///
/// Under the hood t-rust-less only stores SecretVersion's, a Secret is no more (or less)
/// than a group-by view over all SecretVersion's. As a rule a SecretVersion shall never be
/// overwritten or modified once stored. To change a Secret just add a new SecretVersion for it.
///
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretVersion {
  /// Identifier of the secret this version belongs to.
  /// This should be opaque (i.e. not reveal anything about the content whatsoever), e.g. a
  /// random string of sufficient length or some sort of UUID will do fine.
  ///
  /// By the way, as UUID was mentioned: A time-based UUID will reveal the MAC address of the
  /// creator of the Secret as well as when it was created. If you are fine was that, ok,
  /// otherwise do not use this kind of UUID.
  pub secret_id: String,
  /// General type of the Secret (in this version)
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  /// Timestamp of this version. All SecretVersion's of a Secret a sorted by their timestamps,
  /// the last one will be considered the current version.
  pub timestamp: ZeroizeDateTime,
  /// Name/title of the Secret (in this version)
  pub name: String,
  /// List or arbitrary tags for filtering (or just displaying)
  #[serde(default)]
  pub tags: Vec<String>,
  /// List of URLs the Secret might be associated with (most commonly the login page where
  /// the Secret is needed)
  #[serde(default)]
  pub urls: Vec<String>,
  /// Generic list of secret properties. The `secret_type` defines a list of commonly used
  /// property-names for that type.
  pub properties: SecretProperties,
  /// List of attachments.
  #[serde(default)]
  pub attachments: Vec<SecretAttachment>,
  /// If this version of the Secret should be marked as deleted.
  /// As a rule of thumb it is a very bad idea to just delete secret. Maybe it was deleted by
  /// accident, or you might need it for other reasons you have not thought of. Also just
  /// deleting a Secret does not make it unseen. The information that someone (or yourself) has
  /// once seen this secret might be as valuable as the secret itself.
  #[serde(default)]
  pub deleted: bool,
  /// List of recipients that may see this version of the Secret.
  /// Again: Once published, it cannot be made unseen. The only safe way to remove a recipient is
  /// to change the Secret and create a new version without the recipient.
  #[serde(default)]
  pub recipients: Vec<String>,
}

impl SecretVersion {
  pub fn to_entry_builder(&self, mut builder: secret_entry::Builder) -> capnp::Result<()> {
    builder.set_id(&self.secret_id);
    builder.set_timestamp(self.timestamp.timestamp_millis());
    builder.set_name(&self.name);
    builder.set_type(self.secret_type.to_builder());
    set_text_list(builder.reborrow().init_tags(self.tags.len() as u32), &self.tags)?;
    set_text_list(builder.reborrow().init_urls(self.urls.len() as u32), &self.urls)?;
    builder.set_deleted(self.deleted);
    Ok(())
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct PasswordEstimate {
  pub password: String,
  pub inputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Zeroize)]
#[zeroize(drop)]
pub struct PasswordStrength {
  pub entropy: f64,
  pub crack_time: f64,
  pub crack_time_display: String,
  pub score: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretVersionRef {
  pub block_id: String,
  pub timestamp: ZeroizeDateTime,
}

impl SecretVersionRef {
  pub fn from_reader(reader: secret_version_ref::Reader) -> capnp::Result<Self> {
    Ok(SecretVersionRef {
      block_id: reader.get_block_id()?.to_string(),
      timestamp: Utc.timestamp_millis(reader.get_timestamp()).into(),
    })
  }

  pub fn to_builder(&self, mut builder: secret_version_ref::Builder) {
    builder.set_block_id(&self.block_id);
    builder.set_timestamp(self.timestamp.timestamp_millis());
  }
}

impl std::fmt::Display for SecretVersionRef {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.timestamp.format("%Y-%m-%d %H:%M:%S"))
  }
}

/// Representation of a secret with all its versions.
///
/// The is the default view when retrieving a specific secret.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Secret {
  pub id: String,
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub current: SecretVersion,
  pub current_block_id: String,
  pub versions: Vec<SecretVersionRef>,
  pub password_strengths: HashMap<String, PasswordStrength>,
}

impl Zeroize for Secret {
  fn zeroize(&mut self) {
    self.id.zeroize();
    self.secret_type.zeroize();
    self.current.zeroize();
    self.current_block_id.zeroize();
    self.versions.zeroize();
    self.password_strengths.values_mut().for_each(Zeroize::zeroize);
  }
}

impl Drop for Secret {
  fn drop(&mut self) {
    self.zeroize();
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct PasswordGeneratorCharsParam {
  pub num_chars: u8,
  pub include_uppers: bool,
  pub include_numbers: bool,
  pub include_symbols: bool,
  pub require_upper: bool,
  pub require_number: bool,
  pub require_symbol: bool,
  pub exclude_similar: bool,
  pub exclude_ambiguous: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct PasswordGeneratorWordsParam {
  pub num_words: u8,
  pub delim: char,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[serde(rename_all = "lowercase")]
#[zeroize(drop)]
pub enum PasswordGeneratorParam {
  Chars(PasswordGeneratorCharsParam),
  Words(PasswordGeneratorWordsParam),
}

pub fn set_text_list<I, S>(mut text_list: text_list::Builder, texts: I) -> capnp::Result<()>
where
  I: IntoIterator<Item = S>,
  S: AsRef<str>,
{
  for (idx, text) in texts.into_iter().enumerate() {
    text_list.set(idx as u32, capnp::text::new_reader(text.as_ref().as_bytes())?);
  }
  Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct ClipboardProviding {
  pub store_name: String,
  pub block_id: String,
  pub secret_name: String,
  pub property: String,
}
