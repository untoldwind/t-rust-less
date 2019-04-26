use crate::commands::unlock_store;
use crate::error::ExtResult;
use crate::model::import_v1::SecretV1;
use std::fs::File;
use std::io::{stdin, BufRead, BufReader};
use std::sync::Arc;
use t_rust_less_lib::service::TrustlessService;

pub fn import_v1(service: Arc<TrustlessService>, store_name: String, maybe_file_name: Option<&str>) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));

  unlock_store(&secrets_store, &store_name);

  let import_stream: Box<BufRead> = match maybe_file_name {
    Some(file_name) => {
      let file = File::open(file_name).ok_or_exit(format!("Failed opening {}", file_name));
      Box::new(BufReader::new(file))
    }
    None => Box::new(BufReader::new(stdin())),
  };

  for maybe_line in import_stream.lines() {
    let line = maybe_line.ok_or_exit("IO Error");
    let secret = serde_json::from_str::<SecretV1>(&line).ok_or_exit("Invalid format");

    println!("Importing secret {}", secret.id);
  }
}
