use crate::config::Config;
use crate::error::ExtResult;
use std::io;

pub fn list_identities(config: Config) {
  let secrets_store = config.open_secrets_store();
  let identities = secrets_store.identities().ok_or_exit("Failed listing identities: ");

  serde_json::to_writer(io::stdout(), &identities).ok_or_exit("Failed dumping identities: ");
}
