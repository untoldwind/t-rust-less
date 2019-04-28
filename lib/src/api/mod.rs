use std::collections::BTreeMap;
use std::collections::HashMap;

use crate::api_capnp::{self, identity, secret_entry, option, status};
use crate::memguard::weak::{ZeroingBytes, ZeroingString, ZeroingStringExt};
use chrono::{DateTime, TimeZone, Utc};
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

/// Convenient wrapper of a list of SecretEntryMatch'es.
///
/// Also contains a unique list of tags of all secrets (e.g. to support autocompletion)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecretList {
  pub all_tags: Vec<ZeroingString>,
  pub entries: Vec<SecretEntryMatch>,
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
  pub properties: BTreeMap<String, ZeroingString>,
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

/// Convenient wrapper for the current version of a Secret.
///
/// The is the default view when retrieving a specific Secret.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Secret {
  pub id: String,
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub current: SecretVersion,
  pub has_versions: bool,
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
