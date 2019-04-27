use crate::secrets_store::SecretsStore;
use crate::service::{ServiceResult, StoreConfig, TrustlessService};
use crate::service_capnp::service;
use capnp::capability::Promise;
use futures::Future;
use std::sync::Arc;
use std::sync::Mutex;
use tokio::runtime::current_thread;

pub struct RemoteTrustlessService {
  client: service::Client,
  runtime: Mutex<current_thread::Runtime>,
}

impl RemoteTrustlessService {
  pub fn new(client: service::Client, runtime: current_thread::Runtime) -> RemoteTrustlessService {
    RemoteTrustlessService {
      client,
      runtime: Mutex::new(runtime),
    }
  }
}

impl TrustlessService for RemoteTrustlessService {
  fn list_stores(&self) -> ServiceResult<Vec<String>> {
    let mut runtime = self.runtime.lock()?;
    let request = self.client.list_stores_request();
    let result = runtime.block_on(request.send().promise.and_then(|response| {
      let names = response
        .get()?
        .get_store_names()?
        .into_iter()
        .flatten()
        .map(|name| name.to_string())
        .collect::<Vec<String>>();
      Ok(names)
    }))?;

    Ok(result)
  }

  fn set_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    unimplemented!()
  }

  fn get_store_config(&self, name: &str) -> ServiceResult<StoreConfig> {
    unimplemented!()
  }

  fn open_store(&self, name: &str) -> ServiceResult<Arc<SecretsStore>> {
    unimplemented!()
  }

  fn get_default_store(&self) -> ServiceResult<Option<String>> {
    unimplemented!()
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    unimplemented!()
  }
}
