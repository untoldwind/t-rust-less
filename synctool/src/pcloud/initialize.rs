use const_hex::ToHexExt;
use pcloud::{Region, EU_REGION, US_REGION};
use serde::Deserialize;
use sha1::{Digest, Sha1};
use zeroize::Zeroizing;

use crate::error::SyncResult;

pub async fn get_pcloud_token(
  region: Region,
  username: Zeroizing<String>,
  password: Zeroizing<String>,
) -> SyncResult<Zeroizing<String>> {
  let client = pcloud::reqwest::Client::builder()
    .user_agent("t-rust-less-synctool")
    .build()?;
  let base_url = match region {
    Region::Eu => EU_REGION,
    Region::Us => US_REGION,
  };
  let res = client.get(format!("{base_url}/getdigest")).send().await?;
  res.error_for_status_ref()?;
  let digest_body = res.json::<DigestBody>().await?;
  let password_digest = get_password_digest(&digest_body.digest, &username, &password);

  let res = client
    .get(format!("{base_url}/userinfo"))
    .query(&[
      ("getauth", "1"),
      ("username", &username),
      ("passworddigest", &password_digest),
      ("digest", &digest_body.digest),
    ])
    .send()
    .await?;
  res.error_for_status_ref()?;
  let userauth_body = res.json::<UserAuthBody>().await?;

  Ok(userauth_body.auth.to_string().into())
}

pub fn get_password_digest(digest: &str, username: &str, password: &str) -> Zeroizing<String> {
  let mut hasher = Sha1::new();
  hasher.update(username.to_lowercase());
  let user_hash = hasher.finalize_reset().as_slice().encode_hex();
  hasher.update(password);
  hasher.update(user_hash);
  hasher.update(digest);

  hasher.finalize().as_slice().encode_hex().into()
}

#[derive(Debug, Deserialize)]
struct DigestBody {
  digest: String,
}

#[derive(Debug, Deserialize)]
struct UserAuthBody {
  auth: String,
}
