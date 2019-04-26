use crate::commands::unlock_store;
use crate::error::ExtResult;
use crate::model::import_v1::SecretV1;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use std::process;
use std::sync::Arc;
use t_rust_less_lib::api::SecretVersion;
use t_rust_less_lib::service::TrustlessService;

pub fn import_v1(service: Arc<TrustlessService>, store_name: String, maybe_file_name: Option<&str>) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));

  let status = secrets_store.status().ok_or_exit("Get status");

  let import_stream: Box<BufRead> = match maybe_file_name {
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
    unlock_store(&secrets_store, &store_name);
  }

  for maybe_line in import_stream.lines() {
    let line = maybe_line.ok_or_exit("IO Error");
    let secret = serde_json::from_str::<SecretV1>(&line).ok_or_exit("Invalid format");

    eprintln!("Importing secret {}", secret.id);

    for v1_version in secret.versions {
      let version = SecretVersion {
        secret_id: secret.id.to_string(),
        secret_type: secret.secret_type.clone(),
        timestamp: v1_version.timestamp,
        name: v1_version.name,
        tags: v1_version.tags.unwrap_or_default(),
        urls: v1_version.urls.unwrap_or_default(),
        attachments: v1_version.attachments.unwrap_or_default(),
        properties: v1_version.properties,
        deleted: v1_version.deleted,
        recipients: vec![],
      };

      secrets_store.add(version).ok_or_exit("Add secret version")
    }
  }
}
