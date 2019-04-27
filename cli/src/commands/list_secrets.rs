use crate::commands::unlock_store;
use crate::error::ExtResult;
use std::sync::Arc;
use t_rust_less_lib::api::SecretListFilter;
use t_rust_less_lib::service::TrustlessService;

pub fn list_secrets(service: Arc<TrustlessService>, store_name: String) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));

  let status = secrets_store.status().ok_or_exit("Get status");

  if status.locked {
    unlock_store(&secrets_store, &store_name);
  }

  let filter: SecretListFilter = Default::default();
  let list = secrets_store.list(filter).ok_or_exit("List entries");

  for entry in list.entries {
    println!("{:?}", entry);
  }
}
