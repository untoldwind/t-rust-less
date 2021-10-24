use crate::api::{ClipboardProviding, SecretVersion, PROPERTY_TOTP_URL};
use crate::clipboard::SelectionProvider;
use crate::otp::OTPAuthUrl;
use log::{error, info};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct SecretsProvider {
  store_name: String,
  block_id: String,
  secret_version: SecretVersion,
  properties_stack: Vec<String>,
}

impl SecretsProvider {
  pub fn new(store_name: String, block_id: String, secret_version: SecretVersion, properties: &[&str]) -> Self {
    let properties_stack = properties.iter().rev().map(ToString::to_string).collect();
    SecretsProvider {
      store_name,
      block_id,
      secret_version,
      properties_stack,
    }
  }
}

impl SelectionProvider for SecretsProvider {
  fn current_selection(&self) -> Option<ClipboardProviding> {
    self
      .properties_stack
      .last()
      .cloned()
      .map(|property| ClipboardProviding {
        store_name: self.store_name.clone(),
        block_id: self.block_id.clone(),
        secret_name: self.secret_version.name.clone(),
        property,
      })
  }

  fn get_selection_value(&self) -> Option<String> {
    let property = self.properties_stack.last()?;
    let value = self.secret_version.properties.get(property)?;

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
      Some(value.clone())
    }
  }

  fn next_selection(&mut self) {
    self.properties_stack.pop();
  }
}
