use crate::api_capnp::{
  self, identity, option, password_generator_param, password_strength, secret, secret_entry, secret_entry_match,
  secret_list, secret_list_filter, secret_version, status,
};
use capnp::{struct_list, text_list};
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;
use zeroize::Zeroize;

mod capnp_serializable;
mod command;
mod config;
mod event;
mod zeroize_datetime;

#[cfg(test)]
mod tests;

pub use capnp_serializable::*;
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

impl CapnpSerializing for Status {
  type Owned = status::Owned;

  #[allow(clippy::redundant_closure)]
  fn from_reader(reader: status::Reader) -> capnp::Result<Self> {
    Ok(Status {
      locked: reader.get_locked(),
      unlocked_by: read_option(reader.get_unlocked_by()?)?
        .map(|i| Identity::from_reader(i))
        .transpose()?,
      autolock_at: {
        let autolock_at = reader.get_autolock_at();
        if autolock_at == std::i64::MIN {
          None
        } else {
          Some(Utc.timestamp_millis(autolock_at).into())
        }
      },
      version: reader.get_version()?.to_string(),
      autolock_timeout: reader.get_autolock_timeout(),
    })
  }

  fn to_builder(&self, mut builder: status::Builder) -> capnp::Result<()> {
    builder.set_locked(self.locked);
    match &self.unlocked_by {
      Some(identity) => identity.to_builder(builder.reborrow().get_unlocked_by()?.init_some())?,
      None => builder.reborrow().get_unlocked_by()?.set_none(()),
    }
    match &self.autolock_at {
      Some(autolock_at) => builder.set_autolock_at(autolock_at.timestamp_millis()),
      None => builder.set_autolock_at(std::i64::MIN),
    }
    builder.set_version(&self.version);
    builder.set_autolock_timeout(self.autolock_timeout);

    Ok(())
  }
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

impl CapnpSerializing for Identity {
  type Owned = identity::Owned;

  fn from_reader(reader: identity::Reader) -> capnp::Result<Self> {
    Ok(Identity {
      id: reader.get_id()?.to_string(),
      name: reader.get_name()?.to_string(),
      email: reader.get_email()?.to_string(),
      hidden: reader.get_hidden(),
    })
  }

  fn to_builder(&self, mut builder: identity::Builder) -> capnp::Result<()> {
    builder.set_id(&self.id);
    builder.set_name(&self.name);
    builder.set_email(&self.email);
    builder.set_hidden(self.hidden);
    Ok(())
  }
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

  pub fn from_reader(api: api_capnp::SecretType) -> Self {
    match api {
      api_capnp::SecretType::Login => SecretType::Login,
      api_capnp::SecretType::Licence => SecretType::Licence,
      api_capnp::SecretType::Wlan => SecretType::Wlan,
      api_capnp::SecretType::Note => SecretType::Note,
      api_capnp::SecretType::Password => SecretType::Password,
      api_capnp::SecretType::Other => SecretType::Other,
    }
  }

  pub fn to_builder(self) -> api_capnp::SecretType {
    match self {
      SecretType::Login => api_capnp::SecretType::Login,
      SecretType::Licence => api_capnp::SecretType::Licence,
      SecretType::Note => api_capnp::SecretType::Note,
      SecretType::Wlan => api_capnp::SecretType::Wlan,
      SecretType::Password => api_capnp::SecretType::Password,
      SecretType::Other => api_capnp::SecretType::Other,
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

impl CapnpSerializing for SecretListFilter {
  type Owned = secret_list_filter::Owned;

  fn from_reader(reader: secret_list_filter::Reader) -> capnp::Result<Self> {
    Ok(SecretListFilter {
      url: read_option(reader.get_url()?)?.map(ToString::to_string),
      tag: read_option(reader.get_tag()?)?.map(ToString::to_string),
      secret_type: match reader.get_type()?.which()? {
        secret_list_filter::option_type::Some(reader) => Some(SecretType::from_reader(reader?)),
        secret_list_filter::option_type::None(_) => None,
      },
      name: read_option(reader.get_name()?)?.map(ToString::to_string),
      deleted: reader.get_deleted(),
    })
  }

  fn to_builder(&self, mut builder: secret_list_filter::Builder) -> capnp::Result<()> {
    match &self.url {
      Some(url) => builder
        .reborrow()
        .init_url()
        .set_some(capnp::text::new_reader(url.as_bytes())?)?,
      None => builder.reborrow().init_url().set_none(()),
    }
    match &self.tag {
      Some(tag) => builder
        .reborrow()
        .init_tag()
        .set_some(capnp::text::new_reader(tag.as_bytes())?)?,
      None => builder.reborrow().init_tag().set_none(()),
    }
    match &self.secret_type {
      Some(secret_type) => builder.reborrow().init_type().set_some(secret_type.to_builder()),
      None => builder.reborrow().init_type().set_none(()),
    }
    match &self.name {
      Some(name) => builder
        .reborrow()
        .init_name()
        .set_some(capnp::text::new_reader(name.as_bytes())?)?,
      None => builder.reborrow().init_name().set_none(()),
    }
    builder.set_deleted(self.deleted);

    Ok(())
  }
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

impl SecretEntryMatch {
  pub fn from_reader(reader: secret_entry_match::Reader) -> capnp::Result<Self> {
    Ok(SecretEntryMatch {
      entry: SecretEntry::from_reader(reader.get_entry()?)?,
      name_score: reader.get_name_score() as isize,
      name_highlights: reader.get_name_highlights()?.into_iter().map(|h| h as usize).collect(),
      url_highlights: reader.get_url_highlights()?.into_iter().map(|h| h as usize).collect(),
      tags_highlights: reader.get_tags_highlights()?.into_iter().map(|h| h as usize).collect(),
    })
  }

  pub fn to_builder(&self, mut builder: secret_entry_match::Builder) {
    self.entry.to_builder(builder.reborrow().init_entry());
    builder.set_name_score(self.name_score as i64);
    let mut name_highlights = builder
      .reborrow()
      .init_name_highlights(self.name_highlights.len() as u32);
    for (idx, highlight) in self.name_highlights.iter().enumerate() {
      name_highlights.set(idx as u32, *highlight as u64);
    }
    let mut url_highlights = builder.reborrow().init_url_highlights(self.url_highlights.len() as u32);
    for (idx, highlight) in self.url_highlights.iter().enumerate() {
      url_highlights.set(idx as u32, *highlight as u64);
    }
    let mut tags_highlights = builder.init_tags_highlights(self.tags_highlights.len() as u32);
    for (idx, highlight) in self.tags_highlights.iter().enumerate() {
      tags_highlights.set(idx as u32, *highlight as u64);
    }
  }
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

impl CapnpSerializing for SecretList {
  type Owned = secret_list::Owned;

  fn from_reader(reader: secret_list::Reader) -> capnp::Result<Self> {
    Ok(SecretList {
      all_tags: reader
        .get_all_tags()?
        .into_iter()
        .map(|t| t.map(|t| t.to_string()))
        .collect::<capnp::Result<Vec<String>>>()?,
      entries: reader
        .get_entries()?
        .into_iter()
        .map(SecretEntryMatch::from_reader)
        .collect::<capnp::Result<Vec<SecretEntryMatch>>>()?,
    })
  }

  fn to_builder(&self, mut builder: secret_list::Builder) -> capnp::Result<()> {
    set_text_list(
      builder.reborrow().init_all_tags(self.all_tags.len() as u32),
      &self.all_tags,
    )?;
    let mut entries = builder.init_entries(self.entries.len() as u32);

    for (idx, entry) in self.entries.iter().enumerate() {
      entry.to_builder(entries.reborrow().get(idx as u32));
    }

    Ok(())
  }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct SecretProperties(BTreeMap<String, String>);

impl SecretProperties {
  pub fn new(properties: BTreeMap<String, String>) -> Self {
    SecretProperties(properties)
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

  pub fn from_reader(reader: struct_list::Reader<secret_version::property::Owned>) -> capnp::Result<Self> {
    let mut properties = BTreeMap::new();
    for property in reader {
      properties.insert(property.get_key()?.to_string(), property.get_value()?.to_string());
    }

    Ok(SecretProperties(properties))
  }

  pub fn to_builder(&self, mut builder: struct_list::Builder<secret_version::property::Owned>) {
    for (idx, (key, value)) in self.0.iter().enumerate() {
      let mut property = builder.reborrow().get(idx as u32);

      property.set_key(key);
      property.set_value(value);
    }
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

impl SecretAttachment {
  pub fn from_reader(reader: secret_version::attachment::Reader) -> capnp::Result<Self> {
    Ok(SecretAttachment {
      name: reader.get_name()?.to_string(),
      mime_type: reader.get_mime_type()?.to_string(),
      content: reader.get_content()?.to_vec(),
    })
  }

  pub fn to_builder(&self, mut builder: secret_version::attachment::Builder) {
    builder.set_name(&self.name);
    builder.set_mime_type(&self.mime_type);
    builder.set_content(&self.content);
  }
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

impl CapnpSerializing for SecretVersion {
  type Owned = secret_version::Owned;

  fn from_reader(reader: secret_version::Reader) -> capnp::Result<Self> {
    Ok(SecretVersion {
      secret_id: reader.get_secret_id()?.to_string(),
      secret_type: SecretType::from_reader(reader.get_type()?),
      timestamp: Utc.timestamp_millis(reader.get_timestamp()).into(),
      name: reader.get_name()?.to_string(),
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
      properties: SecretProperties::from_reader(reader.get_properties()?)?,
      attachments: reader
        .get_attachments()?
        .into_iter()
        .map(SecretAttachment::from_reader)
        .collect::<capnp::Result<Vec<SecretAttachment>>>()?,
      deleted: reader.get_deleted(),
      recipients: reader
        .get_recipients()?
        .into_iter()
        .map(|u| u.map(|u| u.to_string()))
        .collect::<capnp::Result<Vec<String>>>()?,
    })
  }

  fn to_builder(&self, mut builder: secret_version::Builder) -> capnp::Result<()> {
    builder.set_secret_id(&self.secret_id);
    builder.set_type(self.secret_type.to_builder());
    builder.set_timestamp(self.timestamp.timestamp_millis());
    builder.set_name(&self.name);
    set_text_list(builder.reborrow().init_tags(self.tags.len() as u32), &self.tags)?;
    set_text_list(builder.reborrow().init_urls(self.urls.len() as u32), &self.urls)?;
    self
      .properties
      .to_builder(builder.reborrow().init_properties(self.properties.len() as u32));
    let mut attachments = builder.reborrow().init_attachments(self.attachments.len() as u32);
    for (idx, attachment) in self.attachments.iter().enumerate() {
      attachment.to_builder(attachments.reborrow().get(idx as u32));
    }
    builder.set_deleted(self.deleted);
    set_text_list(
      builder.reborrow().init_recipients(self.recipients.len() as u32),
      &self.recipients,
    )?;

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

impl PasswordStrength {
  pub fn from_reader(reader: password_strength::Reader) -> capnp::Result<Self> {
    Ok(PasswordStrength {
      entropy: reader.get_entropy(),
      crack_time: reader.get_crack_time(),
      crack_time_display: reader.get_crack_time_display()?.to_string(),
      score: reader.get_score(),
    })
  }
  pub fn to_builder(&self, mut builder: password_strength::Builder) {
    builder.set_entropy(self.entropy);
    builder.set_crack_time(self.crack_time);
    builder.set_crack_time_display(&self.crack_time_display);
    builder.set_score(self.score);
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretVersionRef {
  pub block_id: String,
  pub timestamp: ZeroizeDateTime,
}

impl SecretVersionRef {
  pub fn from_reader(reader: secret::version_ref::Reader) -> capnp::Result<Self> {
    Ok(SecretVersionRef {
      block_id: reader.get_block_id()?.to_string(),
      timestamp: Utc.timestamp_millis(reader.get_timestamp()).into(),
    })
  }

  pub fn to_builder(&self, mut builder: secret::version_ref::Builder) {
    builder.set_block_id(&self.block_id);
    builder.set_timestamp(self.timestamp.timestamp_millis());
  }
}

impl std::fmt::Display for SecretVersionRef {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.timestamp.format("%Y-%m-%d %H:%M:%S"))
  }
}

/// Reperentation of a secret with all its versions.
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

impl CapnpSerializing for Secret {
  type Owned = secret::Owned;

  fn from_reader(reader: secret::Reader) -> capnp::Result<Self> {
    Ok(Secret {
      id: reader.get_id()?.to_string(),
      secret_type: SecretType::from_reader(reader.get_type()?),
      current: SecretVersion::from_reader(reader.get_current()?)?,
      current_block_id: reader.get_current_block_id()?.to_string(),
      versions: reader
        .get_versions()?
        .into_iter()
        .map(SecretVersionRef::from_reader)
        .collect::<capnp::Result<Vec<SecretVersionRef>>>()?,
      password_strengths: reader
        .get_password_strengths()?
        .into_iter()
        .map(|estimate| {
          Ok((
            estimate.get_key()?.to_string(),
            PasswordStrength::from_reader(estimate.get_strength()?)?,
          ))
        })
        .collect::<capnp::Result<HashMap<String, PasswordStrength>>>()?,
    })
  }

  fn to_builder(&self, mut builder: secret::Builder) -> capnp::Result<()> {
    builder.set_id(&self.id);
    builder.set_type(self.secret_type.to_builder());
    self.current.to_builder(builder.reborrow().init_current())?;
    builder.set_current_block_id(&self.current_block_id);
    let mut versions = builder.reborrow().init_versions(self.versions.len() as u32);
    for (idx, version) in self.versions.iter().enumerate() {
      version.to_builder(versions.reborrow().get(idx as u32));
    }
    let mut password_strengths = builder
      .reborrow()
      .init_password_strengths(self.password_strengths.len() as u32);
    for (idx, (key, strength)) in self.password_strengths.iter().enumerate() {
      let mut password_strength = password_strengths.reborrow().get(idx as u32);
      password_strength.set_key(key);
      strength.to_builder(password_strength.init_strength());
    }

    Ok(())
  }
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

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct PasswordGeneratorCharsParam {
  pub num_chars: u8,
  pub include_uppers: bool,
  pub include_numbers: bool,
  pub include_symbols: bool,
  pub require_upper: bool,
  pub require_number: bool,
  pub require_symbol: bool,
  pub exlcude_similar: bool,
  pub exclude_ambiguous: bool,
}

impl PasswordGeneratorCharsParam {
  pub fn from_reader(reader: password_generator_param::password_generator_chars_param::Reader) -> capnp::Result<Self> {
    Ok(PasswordGeneratorCharsParam {
      num_chars: reader.get_num_chars(),
      include_uppers: reader.get_include_uppers(),
      include_numbers: reader.get_include_numbers(),
      include_symbols: reader.get_include_symbols(),
      require_upper: reader.get_require_upper(),
      require_number: reader.get_require_number(),
      require_symbol: reader.get_require_symbol(),
      exlcude_similar: reader.get_exlcude_similar(),
      exclude_ambiguous: reader.get_exclude_ambiguous(),
    })
  }

  pub fn to_builder(
    &self,
    mut builder: password_generator_param::password_generator_chars_param::Builder,
  ) -> capnp::Result<()> {
    builder.set_num_chars(self.num_chars);
    builder.set_include_uppers(self.include_uppers);
    builder.set_include_numbers(self.include_numbers);
    builder.set_include_symbols(self.include_symbols);
    builder.set_require_upper(self.require_upper);
    builder.set_require_number(self.require_number);
    builder.set_require_symbol(self.require_symbol);
    builder.set_exlcude_similar(self.exlcude_similar);
    builder.set_exclude_ambiguous(self.exclude_ambiguous);
    Ok(())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct PasswordGeneratorWordsParam {
  pub num_words: u8,
  pub delim: char,
}

impl PasswordGeneratorWordsParam {
  pub fn from_reader(reader: password_generator_param::password_generator_words_param::Reader) -> capnp::Result<Self> {
    Ok(PasswordGeneratorWordsParam {
      num_words: reader.get_num_words(),
      delim: std::char::from_u32(reader.get_delim()).unwrap(),
    })
  }

  pub fn to_builder(
    &self,
    mut builder: password_generator_param::password_generator_words_param::Builder,
  ) -> capnp::Result<()> {
    builder.set_num_words(self.num_words);
    builder.set_delim(self.delim as u32);

    Ok(())
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, Zeroize)]
#[serde(rename_all = "lowercase")]
#[zeroize(drop)]
pub enum PasswordGeneratorParam {
  Chars(PasswordGeneratorCharsParam),
  Words(PasswordGeneratorWordsParam),
}

impl CapnpSerializing for PasswordGeneratorParam {
  type Owned = password_generator_param::Owned;

  fn from_reader(reader: password_generator_param::Reader) -> capnp::Result<Self> {
    match reader.which()? {
      password_generator_param::Chars(inner) => Ok(PasswordGeneratorParam::Chars(
        PasswordGeneratorCharsParam::from_reader(inner?)?,
      )),
      password_generator_param::Words(inner) => Ok(PasswordGeneratorParam::Words(
        PasswordGeneratorWordsParam::from_reader(inner?)?,
      )),
    }
  }

  fn to_builder(&self, builder: password_generator_param::Builder) -> capnp::Result<()> {
    match self {
      PasswordGeneratorParam::Chars(param) => param.to_builder(builder.init_chars()),
      PasswordGeneratorParam::Words(param) => param.to_builder(builder.init_words()),
    }
  }
}

pub fn read_option<T>(reader: option::Reader<T>) -> capnp::Result<Option<<T as capnp::traits::Owned<'_>>::Reader>>
where
  T: for<'c> capnp::traits::Owned<'c>,
{
  match reader.which()? {
    option::Some(inner) => Ok(Some(inner?)),
    option::None(_) => Ok(None),
  }
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
