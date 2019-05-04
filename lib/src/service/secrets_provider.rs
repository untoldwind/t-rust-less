use crate::api::{SecretVersion, PROPERTY_TOTP_URL};
use crate::clipboard::SelectionProvider;
use crate::otp::OTPAuthUrl;
use log::{error, info};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SecretsProvider {
  secret_version: SecretVersion,
  properties_stack: Vec<String>,
}

impl SecretsProvider {
  pub fn new(secret_version: SecretVersion, properties: &[&str]) -> Self {
    let properties_stack = properties.into_iter().rev().map(ToString::to_string).collect();
    SecretsProvider {
      secret_version,
      properties_stack,
    }
  }
}

impl SelectionProvider for SecretsProvider {
  fn get_selection(&mut self) -> Option<String> {
    let property = self.properties_stack.pop()?;
    let value = self.secret_version.properties.get(&property)?;

    if property == PROPERTY_TOTP_URL {
      info!("Providing TOTP of {}", self.secret_version.secret_id);
      match OTPAuthUrl::parse(value) {
        Ok(otpauth) => {
          let (token, _) = otpauth.generate(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs());
          Some(token)
        }
        Err(error) => {
          error!("Invalid OTPAuth url: {}", error);
          None
        }
      }
    } else {
      info!("Providing {} of {}", property, self.secret_version.secret_id);
      Some(value.to_string())
    }
  }
}
