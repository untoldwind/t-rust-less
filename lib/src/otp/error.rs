use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Serialize, Deserialize)]
#[cfg_attr(feature = "with_specta", derive(specta::Type))]
pub enum OTPError {
  #[error("Invalid url: {0}")]
  InvalidUrl(String),
  #[error("Invalid url scheme. Expected otpauth")]
  InvalidScheme,
  #[error("Invalid OTP type. Only totp and hotp are supported")]
  InvalidType,
  #[error("Invalid OTP algorithm. Only SHA1, SHA256, SHA512 are supported")]
  InvalidAlgorithm,
  #[error("Invalid secret")]
  InvalidSecret,
  #[error("Missing required parameter: {0}")]
  MissingParameter(String),
}

pub type OTPResult<T> = Result<T, OTPError>;

error_convert_from!(url::ParseError, OTPError, InvalidUrl(display));
