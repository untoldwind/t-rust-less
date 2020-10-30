use serde_derive::{Deserialize, Serialize};
use t_rust_less_lib::api::{SecretAttachment, SecretProperties, SecretType, ZeroizeDateTime};
use zeroize::Zeroize;

#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct SecretV1 {
  pub id: String,
  #[serde(rename = "type")]
  pub secret_type: SecretType,
  pub versions: Vec<SecretVersionV1>,
}

#[derive(Clone, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct SecretVersionV1 {
  pub timestamp: ZeroizeDateTime,
  pub name: String,
  pub tags: Option<Vec<String>>,
  pub urls: Option<Vec<String>>,
  pub properties: SecretProperties,
  pub attachments: Option<Vec<SecretAttachment>>,
  pub deleted: bool,
}
