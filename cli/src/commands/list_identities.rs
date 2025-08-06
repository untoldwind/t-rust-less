use anyhow::{Context, Result};
use clap::Args;
use std::io;
use std::sync::Arc;
use t_rust_less_lib::service::TrustlessService;

#[derive(Debug, Args)]
pub struct ListIdentitiesCommand {}

impl ListIdentitiesCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) -> Result<()> {
    let secrets_store = service
      .open_store(&store_name)
      .with_context(|| format!("Failed opening store {store_name}: "))?;
    let identities = secrets_store
      .identities()
      .with_context(|| "Failed listing identities: ")?;

    serde_json::to_writer(io::stdout(), &identities).with_context(|| "Failed dumping identities: ")?;

    Ok(())
  }
}
