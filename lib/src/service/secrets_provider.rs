use crate::api::SecretVersion;
use crate::clipboard::SelectionProvider;

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
    self
      .properties_stack
      .pop()
      .and_then(|property| self.secret_version.properties.get(&property))
      .map(ToString::to_string)
  }
}
