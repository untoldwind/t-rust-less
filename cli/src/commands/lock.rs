use anyhow::{Context, Result};
use clap::Args;
use std::sync::Arc;
use t_rust_less_lib::service::TrustlessService;

#[derive(Debug, Args)]
pub struct LockCommand {}

impl LockCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) -> Result<()> {
    let secrets_store = service
      .open_store(&store_name)
      .with_context(|| format!("Failed opening store {store_name}: "))?;

    let status = secrets_store.status().with_context(|| "Get status")?;

    if !status.locked {
      secrets_store.lock().with_context(|| "Lock store")?;
    }

    Ok(())
  }
}
