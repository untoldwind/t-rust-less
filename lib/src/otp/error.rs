use std::fmt;

pub enum OTPError {
  InvalidUrl(String),
  InvalidScheme,
  InvalidType,
  InvalidAlgorithm,
  MissingParameter(String),
}

impl fmt::Display for OTPError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      OTPError::InvalidUrl(error) => write!(f, "Invalid url: {}", error)?,
      OTPError::InvalidScheme => write!(f, "Invalid url scheme. Expected otpauth")?,
      OTPError::InvalidType => write!(f, "Invalid OTP type. Only totp and hotp are supported")?,
      OTPError::InvalidAlgorithm => write!(f, "Invalid OTP algorithm. Only SHA1, SHA256, SHA512 are supported")?,
      OTPError::MissingParameter(name) => write!(f, "Missing required parameter: {}", name)?,
    }

    Ok(())
  }
}

pub type OTPResult<T> = Result<T, OTPError>;

error_convert_from!(url::ParseError, OTPError, InvalidUrl(display));
