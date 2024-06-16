use crate::error::ExtResult;
use clap::Args;
use std::io;
use std::sync::Arc;
use t_rust_less_lib::service::TrustlessService;

#[derive(Debug, Args)]
pub struct ListIdentitiesCommand {}

impl ListIdentitiesCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) {
    let secrets_store = service
      .open_store(&store_name)
      .ok_or_exit(format!("Failed opening store {}: ", store_name));
    let identities = secrets_store.identities().ok_or_exit("Failed listing identities: ");

    serde_json::to_writer(io::stdout(), &identities).ok_or_exit("Failed dumping identities: ");
  }
}
