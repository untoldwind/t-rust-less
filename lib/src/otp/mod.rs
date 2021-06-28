use std::fmt;
use url::{form_urlencoded, Url};

mod error;
mod hotp;
mod totp;

#[cfg(test)]
mod tests;

pub use self::error::*;
use crate::otp::hotp::HOTPGenerator;
use crate::otp::totp::TOTPGenerator;
use std::str::FromStr;
use zeroize::Zeroize;

const OTP_URL_SCHEME: &str = "otpauth";

pub enum OTPType {
  Totp { period: u32 },
  Hotp { counter: u64 },
}

impl fmt::Display for OTPType {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      OTPType::Totp { .. } => write!(f, "totp")?,
      OTPType::Hotp { .. } => write!(f, "hotp")?,
    }
    Ok(())
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OTPAlgorithm {
  SHA1,
  SHA256,
  SHA512,
}

impl fmt::Display for OTPAlgorithm {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      OTPAlgorithm::SHA1 => write!(f, "SHA1")?,
      OTPAlgorithm::SHA256 => write!(f, "SHA256")?,
      OTPAlgorithm::SHA512 => write!(f, "SHA512")?,
    }
    Ok(())
  }
}

#[derive(Zeroize)]
#[zeroize(drop)]
pub struct OTPSecret(Vec<u8>);

impl ToString for OTPSecret {
  fn to_string(&self) -> String {
    data_encoding::BASE32_NOPAD.encode(&self.0)
  }
}

impl FromStr for OTPSecret {
  type Err = OTPError;

  fn from_str(s: &str) -> OTPResult<Self> {
    match data_encoding::BASE32_NOPAD.decode(s.as_bytes()) {
      Ok(bytes) => Ok(OTPSecret(bytes)),
      Err(_) => Err(OTPError::InvalidSecret),
    }
  }
}

pub struct OTPAuthUrl {
  pub otp_type: OTPType,
  pub algorithm: OTPAlgorithm,
  pub digits: u8,
  pub account_name: String,
  pub issuer: Option<String>,
  pub secret: OTPSecret,
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
        OTPType::Totp { period }
      }
      Some("hotp") => {
        let counter = Self::find_required_parameter(&url, "counter")?;
        OTPType::Hotp { counter }
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
    let algorithm = match Self::find_parameter::<String>(&url, "algorithm")?.as_deref() {
      Some("SHA1") | None => OTPAlgorithm::SHA1,
      Some("SHA256") => OTPAlgorithm::SHA256,
      Some("SHA512") => OTPAlgorithm::SHA512,
      Some(_) => return Err(OTPError::InvalidAlgorithm),
    };
    let digits = Self::find_parameter(&url, "digits")?.unwrap_or(6);
    let secret = Self::find_required_parameter(&url, "secret")?;

    Ok(OTPAuthUrl {
      otp_type,
      algorithm,
      digits,
      account_name,
      issuer,
      secret,
    })
  }

  pub fn to_url(&self) -> String {
    let mut result = format!("{}://{}/", OTP_URL_SCHEME, self.otp_type.to_string());

    if let Some(issuer) = &self.issuer {
      result.extend(form_urlencoded::byte_serialize(issuer.as_bytes()));
      result += ":"
    }
    result.extend(form_urlencoded::byte_serialize(self.account_name.as_bytes()));
    result += "?secret=";
    result += &self.secret.to_string();
    match self.otp_type {
      OTPType::Totp { period } if period != 30 => result += &format!("&period={}", period),
      OTPType::Totp { .. } => (),
      OTPType::Hotp { counter } => result += &format!("&counter={}", counter),
    }
    if self.digits != 6 {
      result += &format!("&digits={}", self.digits);
    }
    if let Some(issuer) = &self.issuer {
      result += "&issuer=";
      result.extend(form_urlencoded::byte_serialize(issuer.as_bytes()));
    }
    if self.algorithm != OTPAlgorithm::SHA1 {
      result += &format!("&algorithm={}", self.algorithm);
    }

    result
  }

  pub fn generate(&self, timestamp_or_counter: u64) -> (String, u64) {
    match self.otp_type {
      OTPType::Totp { period } => TOTPGenerator {
        algorithm: self.algorithm,
        digits: self.digits,
        period,
        secret: &self.secret.0,
      }
      .generate(timestamp_or_counter),
      OTPType::Hotp { .. } => HOTPGenerator {
        algorithm: self.algorithm,
        digits: self.digits,
        counter: timestamp_or_counter,
        secret: &self.secret.0,
      }
      .generate(),
    }
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
