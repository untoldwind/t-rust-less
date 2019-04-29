use crate::api_capnp::{
  self, identity, option, secret, secret_entry, secret_entry_match, secret_list, secret_list_filter, secret_version,
  status,
};
use crate::memguard::weak::{ZeroingBytes, ZeroingBytesExt, ZeroingString, ZeroingStringExt};
use capnp::{struct_list, text_list};
use chrono::{DateTime, TimeZone, Utc};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashMap;

/// Status information of a secrets store
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
  pub locked: bool,
  pub unlocked_by: Option<Identity>,
  pub autolock_at: Option<DateTime<Utc>>,
  pub version: String,
}

impl Status {
  pub fn from_reader(reader: status::Reader) -> capnp::Result<Self> {
    Ok(Status {
      locked: reader.get_locked(),
      unlocked_by: read_option(reader.get_unlocked_by()?)?
        .map(Identity::from_reader)
        .transpose()?,
      autolock_at: {
        let autolock_at = reader.get_autolock_at();
        if autolock_at == std::i64::MIN {
          None
        } else {
          Some(Utc.timestamp_millis(autolock_at))
        }
      },
      version: reader.get_version()?.to_string(),
    })
  }

  pub fn to_builder(&self, mut builder: status::Builder) -> capnp::Result<()> {
    builder.set_locked(self.locked);
    match &self.unlocked_by {
      Some(identity) => identity.to_builder(builder.reborrow().get_unlocked_by()?.init_some()),
      None => builder.reborrow().get_unlocked_by()?.set_none(()),
    }
    match &self.autolock_at {
      Some(autolock_at) => builder.set_autolock_at(autolock_at.timestamp_millis()),
      None => builder.set_autolock_at(std::i64::MIN),
    }
    builder.set_version(&self.version);

    Ok(())
  }
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

impl Identity {
  pub fn from_reader(reader: identity::Reader) -> capnp::Result<Self> {
    Ok(Identity {
      id: reader.get_id()?.to_string(),
      name: reader.get_name()?.to_string(),
      email: reader.get_email()?.to_string(),
    })
  }

  pub fn to_builder(&self, mut builder: identity::Builder) {
    builder.set_id(&self.id);
    builder.set_name(&self.name);
    builder.set_email(&self.email);
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

impl SecretType {
  /// Get the commonly used property name that may contain a password.
  ///
  /// The values of these properties are automatically estimated for the strengths.
  pub fn password_properties(&self) -> &[&str] {
    match self {
      SecretType::Login => &["password"],
      SecretType::Note => &[],
      SecretType::Licence => &[],
      SecretType::Wlan => &["password"],
      SecretType::Password => &["password"],
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

  pub fn to_builder(&self) -> api_capnp::SecretType {
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

impl SecretListFilter {
  pub fn from_reader(reader: secret_list_filter::Reader) -> capnp::Result<Self> {
    Ok(SecretListFilter {
      url: read_option(reader.get_url()?)?.map(|u| u.to_string()),
      tag: read_option(reader.get_tag()?)?.map(|u| u.to_string()),
      secret_type: match reader.get_type()?.which()? {
        secret_list_filter::option_type::Some(reader) => Some(SecretType::from_reader(reader?)),
        secret_list_filter::option_type::None(_) => None,
      },
      name: read_option(reader.get_name()?)?.map(|u| u.to_string()),
      deleted: reader.get_deleted(),
    })
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

impl SecretEntry {
  pub fn from_reader(reader: secret_entry::Reader) -> capnp::Result<Self> {
    Ok(SecretEntry {
      id: reader.get_id()?.to_string(),
      timestamp: Utc.timestamp_millis(reader.get_timestamp()),
      name: reader.get_name()?.to_zeroing(),
      secret_type: SecretType::from_reader(reader.get_type()?),
      tags: reader
        .get_tags()?
        .into_iter()
        .map(|t| t.map(|t| t.to_zeroing()))
        .collect::<capnp::Result<Vec<ZeroingString>>>()?,
      urls: reader
        .get_urls()?
        .into_iter()
        .map(|u| u.map(|u| u.to_zeroing()))
        .collect::<capnp::Result<Vec<ZeroingString>>>()?,
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

impl SecretEntryMatch {
  pub fn to_builder(&self, mut builder: secret_entry_match::Builder) {
    self.entry.to_builder(builder.reborrow().init_entry());
    builder.set_name_score(self.name_score as i64);
    let mut name_highlights = builder
      .reborrow()
      .init_name_highlights(self.name_highlights.len() as u32);
    for (idx, highlight) in self.name_highlights.iter().enumerate() {
      name_highlights.set(idx as u32, *highlight as u64);
    }
    let mut url_highlights = builder
      .reborrow()
      .init_url_highlights(self.name_highlights.len() as u32);
    for (idx, highlight) in self.url_highlights.iter().enumerate() {
      url_highlights.set(idx as u32, *highlight as u64);
    }
    let mut tags_highlights = builder.init_tags_highlights(self.name_highlights.len() as u32);
    for (idx, highlight) in self.tags_highlights.iter().enumerate() {
      tags_highlights.set(idx as u32, *highlight as u64);
    }
  }
}

/// Convenient wrapper of a list of SecretEntryMatch'es.
///
/// Also contains a unique list of tags of all secrets (e.g. to support autocompletion)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretList {
  pub all_tags: Vec<ZeroingString>,
  pub entries: Vec<SecretEntryMatch>,
}

impl SecretList {
  pub fn to_builder(&self, mut builder: secret_list::Builder) -> capnp::Result<()> {
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretProperties(BTreeMap<String, ZeroingString>);

impl SecretProperties {
  pub fn new() -> Self {
    SecretProperties(BTreeMap::new())
  }

  pub fn get(&self, name: &str) -> Option<&str> {
    self.0.get(name).map(|s| s.as_str())
  }

  pub fn from_reader(reader: struct_list::Reader<secret_version::property::Owned>) -> capnp::Result<Self> {
    let mut properties = BTreeMap::new();
    for property in reader {
      properties.insert(property.get_key()?.to_string(), property.get_value()?.to_zeroing());
    }

    Ok(SecretProperties(properties))
  }
}

/// Some short of attachment to a secret.
///
/// Be aware that t-rust-less is supposed to be a password store, do not misuse it as a
/// secure document store. Nevertheless, sometimes it might be convenient added some
/// sort of (small) document to a password.
///
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretAttachment {
  name: String,
  mime_type: String,
  content: ZeroingBytes,
}

impl SecretAttachment {
  pub fn from_reader(reader: secret_version::attachment::Reader) -> capnp::Result<Self> {
    Ok(SecretAttachment {
      name: reader.get_name()?.to_string(),
      mime_type: reader.get_mime_type()?.to_string(),
      content: reader.get_content()?.to_zeroing(),
    })
  }
}

/// SecretVersion holds all information of a specific version of a secret.
///
/// Under the hood t-rust-less only stores SecretVersion's, a Secret is no more (or less)
/// than a group-by view over all SecretVersion's. As a rule a SecretVersion shall never be
/// overwritten or modified once stored. To change a Secret just add a new SecretVersion for it.
///
#[derive(Clone, Debug, Serialize, Deserialize)]
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
  pub secret_type: SecretType,
  /// Timestamp of this version. All SecretVersion's of a Secret a sorted by their timestamps,
  /// the last one will be considered the current version.
  pub timestamp: DateTime<Utc>,
  /// Name/title of the Secret (in this version)
  pub name: ZeroingString,
  /// List or arbitrary tags for filtering (or just displaying)
  #[serde(default)]
  pub tags: Vec<ZeroingString>,
  /// List of URLs the Secret might be associated with (most commonly the login page where
  /// the Secret is needed)
  #[serde(default)]
  pub urls: Vec<ZeroingString>,
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
  pub recipients: Vec<ZeroingString>,
}

impl SecretVersion {
  pub fn from_reader(reader: secret_version::Reader) -> capnp::Result<Self> {
    Ok(SecretVersion {
      secret_id: reader.get_secret_id()?.to_string(),
      secret_type: SecretType::from_reader(reader.get_type()?),
      timestamp: Utc.timestamp_millis(reader.get_timestamp()),
      name: reader.get_name()?.to_zeroing(),
      tags: reader
        .get_tags()?
        .into_iter()
        .map(|t| t.map(|t| t.to_zeroing()))
        .collect::<capnp::Result<Vec<ZeroingString>>>()?,
      urls: reader
        .get_urls()?
        .into_iter()
        .map(|u| u.map(|u| u.to_zeroing()))
        .collect::<capnp::Result<Vec<ZeroingString>>>()?,
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
        .map(|u| u.map(|u| u.to_zeroing()))
        .collect::<capnp::Result<Vec<ZeroingString>>>()?,
    })
  }

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordEstimate {
  pub password: ZeroingString,
  pub inputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasswordStrength {
  pub entropy: f64,
  pub crack_time: u64,
  pub crack_time_display: String,
  pub score: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecretVersionRef {
  pub block_id: String,
  pub timestamp: DateTime<Utc>,
}

impl SecretVersionRef {
  pub fn from_reader(reader: secret::version_ref::Reader) -> capnp::Result<Self> {
    Ok(SecretVersionRef {
      block_id: reader.get_block_id()?.to_string(),
      timestamp: Utc.timestamp_millis(reader.get_timestamp()),
    })
  }

  pub fn to_builder(&self, mut builder: secret::version_ref::Builder) {
    builder.set_block_id(&self.block_id);
    builder.set_timestamp(self.timestamp.timestamp_millis());
  }
}

/// Convenient wrapper for the current version of a Secret.
///
/// The is the default view when retrieving a specific Secret.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Secret {
  pub id: String,
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub current: SecretVersion,
  pub versions: Vec<SecretVersionRef>,
  pub password_strengths: HashMap<String, PasswordStrength>,
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

fn set_text_list<I, S>(mut text_list: text_list::Builder, texts: I) -> capnp::Result<()>
where
  I: IntoIterator<Item = S>,
  S: AsRef<str>,
{
  for (idx, text) in texts.into_iter().enumerate() {
    text_list.set(idx as u32, capnp::text::new_reader(text.as_ref().as_bytes())?);
  }
  Ok(())
}
