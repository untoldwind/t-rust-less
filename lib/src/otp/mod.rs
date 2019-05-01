use url::Url;

mod error;

pub use self::error::*;
use std::str::FromStr;

const OTP_URL_SCHEME: &str = "otpauth";

pub enum OTPType {
  TOTP { period: u32 },
  HOTP { counter: u64 },
}

pub enum OTPAlgorithm {
  SHA1,
  SHA256,
  SHA512,
}

pub struct OTPAuthUrl {
  pub otp_type: OTPType,
  pub algorithm: OTPAlgorithm,
  pub digits: u8,
  pub account_name: String,
  pub issuer: Option<String>,
}

impl OTPAuthUrl {
  pub fn parse<S: AsRef<str>>(url_str: S) -> OTPResult<OTPAuthUrl> {
    let url = Url::parse(url_str.as_ref())?;
    if url.scheme() != OTP_URL_SCHEME {
      return Err(OTPError::InvalidScheme);
    }
    let otp_type = match url.host_str() {
      Some("totp") => {
        let period = Self::find_parameter(&url, "period")?.unwrap_or(30);
        OTPType::TOTP { period }
      }
      Some("hotp") => {
        let counter = Self::find_required_parameter(&url, "counter")?;
        OTPType::HOTP { counter }
      }
      _ => return Err(OTPError::InvalidType),
    };
    let mut issuer = Self::find_parameter::<String>(&url, "issuer")?;
    let mut account_name = String::new();
    if url.path().is_empty() {
      return Err(OTPError::MissingParameter("accountname".to_string()));
    } else {
      let mut parts = url.path()[1..].split(':');
      if let Some(issuer_or_account) = parts.next() {
        account_name = issuer_or_account.to_string();
      }
      if let Some(account) = parts.next() {
        issuer = Some(account_name);
        account_name = account.to_string();
      }
    }
    let algorithm = match Self::find_parameter::<String>(&url, "algorithm")?
      .as_ref()
      .map(String::as_str)
    {
      Some("SHA1") | None => OTPAlgorithm::SHA1,
      Some("SHA256") => OTPAlgorithm::SHA256,
      Some("SHA512") => OTPAlgorithm::SHA512,
      Some(_) => return Err(OTPError::InvalidAlgorithm),
    };
    let digits = Self::find_parameter(&url, "digits")?.unwrap_or(6);

    Ok(OTPAuthUrl {
      otp_type,
      algorithm,
      digits,
      account_name,
      issuer,
    })
  }

  fn find_parameter<T: FromStr>(url: &Url, name: &str) -> OTPResult<Option<T>> {
    match url.query_pairs().find(|(key, _)| key == name) {
      Some((_, value)) => {
        let t = value
          .parse::<T>()
          .map_err(|_| OTPError::MissingParameter(name.to_string()))?;
        Ok(Some(t))
      }
      None => Ok(None),
    }
  }

  fn find_required_parameter<T: FromStr>(url: &Url, name: &str) -> OTPResult<T> {
    Self::find_parameter(url, name)?.ok_or_else(|| OTPError::MissingParameter(name.to_string()))
  }
}
