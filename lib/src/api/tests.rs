use crate::api::{
  CapnpSerializable, Identity, PasswordStrength, Secret, SecretAttachment, SecretEntry, SecretEntryMatch, SecretList,
  SecretListFilter, SecretProperties, SecretType, SecretVersion, SecretVersionRef, Status, ZeroizeDateTime,
};
use chrono::{TimeZone, Utc};
use quickcheck::{quickcheck, Arbitrary, Gen};
use std::collections::{BTreeMap, HashMap};

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

#[test]
fn identity_capnp_serialization() {
  fn check_serialize(identity: Identity) -> bool {
    let raw = identity.clone().serialize_capnp().unwrap();
    let deserialized = Identity::deserialize_capnp(&raw).unwrap();

    identity == deserialized
  }

  quickcheck(check_serialize as fn(Identity) -> bool);
}

#[test]
fn status_capnp_serialization() {
  fn check_serialize(status: Status) -> bool {
    let raw = status.clone().serialize_capnp().unwrap();
    let deserialized = Status::deserialize_capnp(&raw).unwrap();

    status == deserialized
  }

  quickcheck(check_serialize as fn(Status) -> bool);
}

#[test]
fn secret_list_filter_capnp_serialization() {
  fn check_serialize(filter: SecretListFilter) -> bool {
    let raw = filter.clone().serialize_capnp().unwrap();
    let deserialized = SecretListFilter::deserialize_capnp(&raw).unwrap();

    filter == deserialized
  }

  quickcheck(check_serialize as fn(SecretListFilter) -> bool);
}

#[test]
fn secret_list_capnp_serialization() {
  fn check_serialize(list: SecretList) -> bool {
    let raw = list.clone().serialize_capnp().unwrap();
    let deserialized = SecretList::deserialize_capnp(&raw).unwrap();

    list == deserialized
  }

  quickcheck(check_serialize as fn(SecretList) -> bool);
}

#[test]
fn secret_version_capnp_serialization() {
  fn check_serialize(secret_version: SecretVersion) -> bool {
    let raw = secret_version.clone().serialize_capnp().unwrap();
    let deserialized = SecretVersion::deserialize_capnp(&raw).unwrap();

    secret_version == deserialized
  }

  quickcheck(check_serialize as fn(SecretVersion) -> bool);
}

#[test]
fn secret_capnp_serialization() {
  fn check_serialize(secret: Secret) -> bool {
    let raw = secret.clone().serialize_capnp().unwrap();
    let deserialized = Secret::deserialize_capnp(&raw).unwrap();

    secret == deserialized
  }

  quickcheck(check_serialize as fn(Secret) -> bool);
}
