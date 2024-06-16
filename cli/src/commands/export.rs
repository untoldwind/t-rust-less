use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Args;
use t_rust_less_lib::{api::SecretListFilter, service::TrustlessService};

use crate::error::ExtResult;

use super::{tui::create_tui, unlock_store};

#[derive(Debug, Args)]
pub struct ExportCommand {
  #[clap(help = "File to export to. If not set export will write to stdout")]
  pub file: Option<String>,
}

impl ExportCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) -> Result<()> {
    let secrets_store = service
      .open_store(&store_name)
      .with_context(|| format!("Failed opening store {}: ", store_name))?;
    let status = secrets_store.status().ok_or_exit("Get status");

    if status.locked {
      let mut siv = create_tui();
      unlock_store(&mut siv, &secrets_store, &store_name)?;
      siv.quit();
    }

    let list = secrets_store.list(&SecretListFilter {
      name: None,
      url: None,
      tag: None,
      ..Default::default()
    })?;

    for entry_match in &list.entries {
      let secret = secrets_store
        .get(&entry_match.entry.id)
        .with_context(|| format!("Get entry {} {}", entry_match.entry.id, entry_match.entry.name))?;

      println!("{}", secret.id);
    }

    Ok(())
  }
}
