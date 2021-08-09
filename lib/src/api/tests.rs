use crate::{
  api::{
    Identity, PasswordStrength, Secret, SecretAttachment, SecretEntry, SecretEntryMatch, SecretList, SecretListFilter,
    SecretProperties, SecretType, SecretVersion, SecretVersionRef, Status, ZeroizeDateTime,
  },
  memguard::SecretBytes,
};
use chrono::{TimeZone, Utc};
use quickcheck::{quickcheck, Arbitrary, Gen};
use std::collections::{BTreeMap, HashMap};

use super::{Command, PasswordGeneratorCharsParam, PasswordGeneratorParam, PasswordGeneratorWordsParam, StoreConfig};
use crate::memguard::ZeroizeBytesBuffer;

impl Arbitrary for Identity {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    Identity {
      id: String::arbitrary(g),
      name: String::arbitrary(g),
      email: String::arbitrary(g),
      hidden: bool::arbitrary(g),
    }
  }
}

impl Arbitrary for ZeroizeDateTime {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    ZeroizeDateTime::from(Utc.timestamp_millis(u32::arbitrary(g) as i64))
  }
}

impl Arbitrary for Status {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    Status {
      locked: bool::arbitrary(g),
      unlocked_by: Option::arbitrary(g),
      autolock_at: Option::arbitrary(g),
      version: String::arbitrary(g),
      autolock_timeout: u64::arbitrary(g),
    }
  }
}

impl Arbitrary for SecretType {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    match g.next_u32() % 6 {
      0 => SecretType::Login,
      1 => SecretType::Note,
      2 => SecretType::Licence,
      3 => SecretType::Wlan,
      4 => SecretType::Password,
      _ => SecretType::Other,
    }
  }
}

impl Arbitrary for SecretListFilter {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretListFilter {
      url: Option::arbitrary(g),
      tag: Option::arbitrary(g),
      secret_type: Option::arbitrary(g),
      name: Option::arbitrary(g),
      deleted: bool::arbitrary(g),
    }
  }
}

impl Arbitrary for SecretEntry {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretEntry {
      id: String::arbitrary(g),
      name: String::arbitrary(g),
      secret_type: SecretType::arbitrary(g),
      tags: Vec::arbitrary(g),
      urls: Vec::arbitrary(g),
      timestamp: ZeroizeDateTime::arbitrary(g),
      deleted: bool::arbitrary(g),
    }
  }
}

impl Arbitrary for SecretEntryMatch {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretEntryMatch {
      entry: SecretEntry::arbitrary(g),
      name_score: isize::arbitrary(g),
      name_highlights: Vec::arbitrary(g),
      url_highlights: Vec::arbitrary(g),
      tags_highlights: Vec::arbitrary(g),
    }
  }
}

impl Arbitrary for SecretList {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretList {
      all_tags: Vec::arbitrary(g),
      entries: vec![SecretEntryMatch::arbitrary(g)],
    }
  }
}

impl Arbitrary for SecretAttachment {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretAttachment {
      name: String::arbitrary(g),
      mime_type: String::arbitrary(g),
      content: Vec::arbitrary(g),
    }
  }
}

impl Arbitrary for SecretProperties {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    let keys = Vec::<String>::arbitrary(g);
    let mut properties = BTreeMap::new();

    for key in keys {
      properties.insert(key, String::arbitrary(g));
    }

    SecretProperties::new(properties)
  }
}

impl Arbitrary for SecretVersion {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretVersion {
      secret_id: String::arbitrary(g),
      secret_type: SecretType::arbitrary(g),
      timestamp: ZeroizeDateTime::arbitrary(g),
      name: String::arbitrary(g),
      tags: Vec::arbitrary(g),
      urls: Vec::arbitrary(g),
      properties: SecretProperties::arbitrary(g),
      attachments: Vec::arbitrary(g),
      deleted: bool::arbitrary(g),
      recipients: Vec::arbitrary(g),
    }
  }
}

impl Arbitrary for SecretVersionRef {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretVersionRef {
      block_id: String::arbitrary(g),
      timestamp: ZeroizeDateTime::arbitrary(g),
    }
  }
}

impl Arbitrary for PasswordStrength {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    PasswordStrength {
      entropy: f64::arbitrary(g),
      crack_time: f64::arbitrary(g),
      crack_time_display: String::arbitrary(g),
      score: u8::arbitrary(g),
    }
  }
}

impl Arbitrary for Secret {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    Secret {
      id: String::arbitrary(g),
      secret_type: SecretType::arbitrary(g),
      current: SecretVersion::arbitrary(g),
      current_block_id: String::arbitrary(g),
      versions: Vec::arbitrary(g),
      password_strengths: HashMap::arbitrary(g),
    }
  }
}

impl Arbitrary for StoreConfig {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    StoreConfig {
      name: String::arbitrary(g),
      store_url: String::arbitrary(g),
      client_id: String::arbitrary(g),
      autolock_timeout_secs: u64::arbitrary(g),
      default_identity_id: Option::arbitrary(g),
    }
  }
}

impl Arbitrary for PasswordGeneratorParam {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    match g.next_u32() % 2 {
      0 => PasswordGeneratorParam::Chars(PasswordGeneratorCharsParam {
        num_chars: u8::arbitrary(g),
        include_uppers: bool::arbitrary(g),
        include_numbers: bool::arbitrary(g),
        include_symbols: bool::arbitrary(g),
        require_upper: bool::arbitrary(g),
        require_number: bool::arbitrary(g),
        require_symbol: bool::arbitrary(g),
        exlcude_similar: bool::arbitrary(g),
        exclude_ambiguous: bool::arbitrary(g),
      }),
      _ => PasswordGeneratorParam::Words(PasswordGeneratorWordsParam {
        num_words: u8::arbitrary(g),
        delim: char::arbitrary(g),
      }),
    }
  }
}

impl Arbitrary for SecretBytes {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    SecretBytes::from(Vec::arbitrary(g))
  }
}

impl Arbitrary for Command {
  fn arbitrary<G: Gen>(g: &mut G) -> Self {
    match g.next_u32() % 24 {
      0 => Command::ListStores,
      1 => Command::UpsertStoreConfig(StoreConfig::arbitrary(g)),
      2 => Command::DeleteStoreConfig(String::arbitrary(g)),
      3 => Command::GetDefaultStore,
      4 => Command::SetDefaultStore(String::arbitrary(g)),
      5 => Command::GenerateId,
      6 => Command::GeneratePassword(PasswordGeneratorParam::arbitrary(g)),
      7 => Command::PollEvents(u64::arbitrary(g)),

      8 => Command::Status(String::arbitrary(g)),
      9 => Command::Lock(String::arbitrary(g)),
      10 => Command::Unlock {
        store_name: String::arbitrary(g),
        identity_id: String::arbitrary(g),
        passphrase: SecretBytes::arbitrary(g),
      },
      11 => Command::Identities(String::arbitrary(g)),
      12 => Command::AddIdentity {
        store_name: String::arbitrary(g),
        identity: Identity::arbitrary(g),
        passphrase: SecretBytes::arbitrary(g),
      },
      13 => Command::ChangePassphrase {
        store_name: String::arbitrary(g),
        passphrase: SecretBytes::arbitrary(g),
      },
      14 => Command::List {
        store_name: String::arbitrary(g),
        filter: SecretListFilter::arbitrary(g),
      },
      15 => Command::UpdateIndex(String::arbitrary(g)),
      16 => Command::Add {
        store_name: String::arbitrary(g),
        secret_version: SecretVersion::arbitrary(g),
      },
      17 => Command::Get {
        store_name: String::arbitrary(g),
        secret_id: String::arbitrary(g),
      },
      18 => Command::GetVersion {
        store_name: String::arbitrary(g),
        block_id: String::arbitrary(g),
      },

      19 => Command::SecretToClipboard {
        store_name: String::arbitrary(g),
        block_id: String::arbitrary(g),
        properties: Vec::arbitrary(g),
        display_name: String::arbitrary(g),
      },
      20 => Command::ClipboardIsDone,
      21 => Command::ClipboardCurrentlyProviding,
      22 => Command::ClipboardProvideNext,
      _ => Command::ClipboardDestroy,
    }
  }
}

#[test]
fn identity_capnp_serialization() {
  fn check_serialize(identity: Identity) -> bool {
    let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
    rmp_serde::encode::write_named(&mut buf, &identity).unwrap();
    let deserialized: Identity = rmp_serde::from_read_ref(&buf).unwrap();

    identity == deserialized
  }

  quickcheck(check_serialize as fn(Identity) -> bool);
}

#[test]
fn status_capnp_serialization() {
  fn check_serialize(status: Status) -> bool {
    let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
    rmp_serde::encode::write_named(&mut buf, &status).unwrap();
    let deserialized: Status = rmp_serde::from_read_ref(&buf).unwrap();

    status == deserialized
  }

  quickcheck(check_serialize as fn(Status) -> bool);
}

#[test]
fn secret_list_filter_capnp_serialization() {
  fn check_serialize(filter: SecretListFilter) -> bool {
    let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
    rmp_serde::encode::write_named(&mut buf, &filter).unwrap();
    let deserialized: SecretListFilter = rmp_serde::from_read_ref(&buf).unwrap();

    filter == deserialized
  }

  quickcheck(check_serialize as fn(SecretListFilter) -> bool);
}

#[test]
fn secret_list_capnp_serialization() {
  fn check_serialize(list: SecretList) -> bool {
    let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
    rmp_serde::encode::write_named(&mut buf, &list).unwrap();
    let deserialized: SecretList = rmp_serde::from_read_ref(&buf).unwrap();

    list == deserialized
  }

  quickcheck(check_serialize as fn(SecretList) -> bool);
}

#[test]
fn secret_version_capnp_serialization() {
  fn check_serialize(secret_version: SecretVersion) -> bool {
    let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
    rmp_serde::encode::write_named(&mut buf, &secret_version).unwrap();
    let deserialized: SecretVersion = rmp_serde::from_read_ref(&buf).unwrap();

    secret_version == deserialized
  }

  quickcheck(check_serialize as fn(SecretVersion) -> bool);
}

#[test]
fn secret_capnp_serialization() {
  fn check_serialize(secret: Secret) -> bool {
    let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
    rmp_serde::encode::write_named(&mut buf, &secret).unwrap();
    let deserialized: Secret = rmp_serde::from_read_ref(&buf).unwrap();

    secret == deserialized
  }

  quickcheck(check_serialize as fn(Secret) -> bool);
}

#[test]
fn command_serialization() {
  fn check_serialize(command: Command) -> bool {
    let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
    rmp_serde::encode::write_named(&mut buf, &command).unwrap();
    let deserialized: Command = rmp_serde::from_read_ref(&buf).unwrap();

    command == deserialized
  }

  quickcheck(check_serialize as fn(Command) -> bool);
}
