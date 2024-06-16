use crate::error::ExtResult;
use clap::Args;
use std::sync::Arc;
use t_rust_less_lib::service::TrustlessService;

#[derive(Debug, Args)]
pub struct LockCommand {}

impl LockCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) {
    let secrets_store = service
      .open_store(&store_name)
      .ok_or_exit(format!("Failed opening store {}: ", store_name));

    let status = secrets_store.status().ok_or_exit("Get status");

    if !status.locked {
      secrets_store.lock().ok_or_exit("Lock store");
    }
  }
}
