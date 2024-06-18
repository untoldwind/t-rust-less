use serde::{Deserialize, Serialize};
use t_rust_less_lib::api::{SecretAttachment, SecretProperties, SecretType, SecretVersion, ZeroizeDateTime};
use zeroize::Zeroize;

#[derive(Debug, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct SecretV2 {
  pub id: String,
  pub current: SecretVersionV2,
  pub versions: Vec<SecretVersionV2>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct SecretVersionV2 {
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub timestamp: ZeroizeDateTime,
  pub name: String,
  #[serde(default)]
  pub tags: Vec<String>,
  #[serde(default)]
  pub urls: Vec<String>,
  pub properties: SecretProperties,
  #[serde(default)]
  pub attachments: Vec<SecretAttachment>,
  #[serde(default)]
  pub deleted: bool,
  #[serde(default)]
  pub recipients: Vec<String>,
}

impl From<&SecretVersion> for SecretVersionV2 {
  fn from(value: &SecretVersion) -> Self {
    SecretVersionV2 {
      secret_type: value.secret_type,
      timestamp: value.timestamp,
      name: value.name.clone(),
      tags: value.tags.clone(),
      urls: value.urls.clone(),
      properties: value.properties.clone(),
      attachments: value.attachments.clone(),
      deleted: value.deleted,
      recipients: value.recipients.clone(),
    }
  }
}
