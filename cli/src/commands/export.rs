use std::{
  fs::File,
  io::{stdout, Write},
  sync::Arc,
};

use anyhow::{Context, Result};
use clap::Args;
use t_rust_less_lib::{api::SecretListFilter, service::TrustlessService};

use crate::{error::ExtResult, model::import_v2::SecretV2};

use super::{tui::create_tui, unlock_store};

#[derive(Debug, Args)]
pub struct ExportCommand {
  #[clap(help = "File to export to. If not set export will write to stdout")]
  pub file: Option<String>,

  #[clap(long)]
  pub include_deleted: bool,

  #[clap(long)]
  pub include_version: bool,
}

impl ExportCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) -> Result<()> {
    let secrets_store = service
      .open_store(&store_name)
      .with_context(|| format!("Failed opening store {store_name}: "))?;
    let status = secrets_store.status().ok_or_exit("Get status");

    if status.locked {
      let mut siv = create_tui();
      unlock_store(&mut siv, &secrets_store, &store_name)?;
      siv.quit();
    }

    let mut filters = vec![SecretListFilter {
      name: None,
      url: None,
      tag: None,
      ..Default::default()
    }];

    if self.include_deleted {
      filters.push(SecretListFilter {
        name: None,
        url: None,
        tag: None,
        deleted: true,
        ..Default::default()
      })
    }

    let mut export_stream: Box<dyn Write> = match &self.file {
      Some(file_name) => {
        let file = File::open(file_name).with_context(|| format!("Failed opening {file_name}"))?;
        Box::new(file)
      }
      None => Box::new(stdout()),
    };

    for filter in &filters {
      let list = secrets_store.list(filter)?;

      for entry_match in &list.entries {
        let secret = secrets_store
          .get(&entry_match.entry.id)
          .with_context(|| format!("Get entry {} {}", entry_match.entry.id, entry_match.entry.name))?;

        let mut service_v2 = SecretV2 {
          id: secret.id.clone(),
          current: (&secret.current).into(),
          versions: vec![],
        };

        if self.include_version {
          for version_ref in &secret.versions {
            let version = secrets_store.get_version(&version_ref.block_id).with_context(|| {
              format!(
                "Get entry version {} {} {:?}",
                entry_match.entry.id, entry_match.entry.name, &version_ref.timestamp
              )
            })?;

            service_v2.versions.push((&version).into());
          }
        }

        serde_json::to_writer(&mut export_stream, &service_v2)?;
        writeln!(&mut export_stream)?;
      }
    }
    export_stream.flush()?;

    Ok(())
  }
}
