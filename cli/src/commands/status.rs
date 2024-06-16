use anyhow::{Context, Result};
use atty::Stream;
use clap::Args;
use crossterm_style::{style, Color};
use std::sync::Arc;
use t_rust_less_lib::service::TrustlessService;

#[derive(Debug, Args)]
pub struct StatusCommand {}

impl StatusCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) -> Result<()> {
    let secrets_store = service
      .open_store(&store_name)
      .with_context(|| format!("Failed opening store {}: ", store_name))?;
    let status = secrets_store.status().with_context(|| "Get status")?;

    if atty::is(Stream::Stdout) {
      println!();
      println!("Client version: {}", style(env!("CARGO_PKG_VERSION")).with(Color::Cyan));
      println!("Store version : {}", style(status.version.clone()).with(Color::Cyan));
      println!(
        "Status        : {}",
        if status.locked {
          style("Locked").with(Color::Green)
        } else {
          style("Unlocked").with(Color::Red)
        }
      )
    } else {
      println!("Client version: {}", env!("CARGO_PKG_VERSION"));
      println!("Store version : {}", status.version);
    }

    Ok(())
  }
}
