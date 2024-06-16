use crate::commands::tui::create_tui;
use crate::commands::unlock_store;
use crate::error::ExtResult;
use crate::model::import_v1::SecretV1;
use clap::Args;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use std::process;
use std::sync::Arc;
use t_rust_less_lib::api::SecretVersion;
use t_rust_less_lib::service::TrustlessService;

#[derive(Debug, Args)]
pub struct ImportCommand {
  #[clap(long, help = "Import V1 format (from original trustless)")]
  pub v1: bool,

  #[clap(help = "File to import. If not set import will read from stdin")]
  pub file: Option<String>,
}

impl ImportCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) {
    if self.v1 {
      import_v1(service, store_name, self.file);
    } else {
      println!("Only v1 import supported yet");
      process::exit(1)
    }
  }
}

pub fn import_v1(service: Arc<dyn TrustlessService>, store_name: String, maybe_file_name: Option<String>) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));

  let status = secrets_store.status().ok_or_exit("Get status");

  let import_stream: Box<dyn BufRead> = match &maybe_file_name {
    Some(file_name) => {
      let file = File::open(file_name).ok_or_exit(format!("Failed opening {}", file_name));
      Box::new(BufReader::new(file))
    }
    None => {
      if status.locked {
        eprintln!("Store is locked! Cannot unlock store when importing from stdin (duh).");
        process::exit(1);
      }
      Box::new(BufReader::new(stdin()))
    }
  };

  if status.locked {
    let mut siv = create_tui();
    unlock_store(&mut siv, &secrets_store, &store_name);
  }

  for maybe_line in import_stream.lines() {
    let line = maybe_line.ok_or_exit("IO Error");
    let mut secret = serde_json::from_str::<SecretV1>(&line).ok_or_exit("Invalid format");

    eprintln!("Importing secret {}", secret.id);

    for v1_version in secret.versions.iter_mut() {
      let version = SecretVersion {
        secret_id: secret.id.to_string(),
        secret_type: secret.secret_type,
        timestamp: v1_version.timestamp,
        name: v1_version.name.clone(),
        tags: v1_version.tags.take().unwrap_or_default(),
        urls: v1_version.urls.take().unwrap_or_default(),
        attachments: v1_version.attachments.take().unwrap_or_default(),
        properties: v1_version.properties.clone(),
        deleted: v1_version.deleted,
        recipients: vec![],
      };

      secrets_store.add(version).ok_or_exit("Add secret version");
    }
  }

  secrets_store.update_index().ok_or_exit("Index update");
}
