use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status, read_option};
use crate::api_capnp::{secrets_store, service};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreResult, SecretsStore};
use crate::service::{ServiceResult, StoreConfig, TrustlessService};
use futures::Future;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::current_thread;

pub struct RemoteTrustlessService {
  client: service::Client,
  runtime: Rc<RefCell<current_thread::Runtime>>,
}

impl RemoteTrustlessService {
  pub fn new(client: service::Client, runtime: current_thread::Runtime) -> RemoteTrustlessService {
    RemoteTrustlessService {
      client,
      runtime: Rc::new(RefCell::new(runtime)),
    }
  }
}

impl TrustlessService for RemoteTrustlessService {
  fn list_stores(&self) -> ServiceResult<Vec<String>> {
    let mut runtime = self.runtime.borrow_mut();
    let request = self.client.list_stores_request();
    let result = runtime.block_on(request.send().promise.and_then(|response| {
      let names = response
        .get()?
        .get_store_names()?
        .into_iter()
        .map(|name| name.map(|n| n.to_string()))
        .collect::<capnp::Result<Vec<String>>>()?;
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
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.open_store_request();
    request.get().set_store_name(name);
    let store_client = runtime.block_on(
      request
        .send()
        .promise
        .and_then(|response| Ok(response.get()?.get_store()?)),
    )?;

    Ok(Arc::new(RemoteSecretsStore::new(store_client, self.runtime.clone())?))
  }

  fn get_default_store(&self) -> ServiceResult<Option<String>> {
    let mut runtime = self.runtime.borrow_mut();
    let request = self.client.get_default_store_request();
    let result = runtime.block_on(
      request
        .send()
        .promise
        .and_then(|response| Ok(read_option(response.get()?.get_default_store()?)?.map(|s| s.to_string()))),
    )?;

    Ok(result)
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    unimplemented!()
  }
}

pub struct RemoteSecretsStore {
  client: secrets_store::Client,
  runtime: Rc<RefCell<current_thread::Runtime>>,
}

impl RemoteSecretsStore {
  fn new(
    client: secrets_store::Client,
    runtime: Rc<RefCell<current_thread::Runtime>>,
  ) -> ServiceResult<RemoteSecretsStore> {
    Ok(RemoteSecretsStore { client, runtime })
  }
}

impl SecretsStore for RemoteSecretsStore {
  fn status(&self) -> SecretStoreResult<Status> {
    let mut runtime = self.runtime.borrow_mut();
    let request = self.client.status_request();
    let result = runtime.block_on(request.send().promise.and_then(|response| {
      Ok(Status::from_reader(response.get()?.get_status()?)?)
    }))?;

    Ok(result)
  }

  fn lock(&self) -> SecretStoreResult<()> {
    let mut runtime = self.runtime.borrow_mut();
    let request = self.client.lock_request();

    runtime.block_on(request.send().promise.and_then(|response| {
      response.get()?;

      Ok(())
    }))?;

    Ok(())
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.unlock_request();
    request.get().set_passphrase(&passphrase.borrow());

    runtime.block_on(request.send().promise.and_then(|response| {
      response.get()?;

      Ok(())
    }))?;

    Ok(())
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    let mut runtime = self.runtime.borrow_mut();
    let request = self.client.identities_request();
    let result = runtime.block_on(request.send().promise.and_then(|response| {
      let names = response
        .get()?
        .get_identities()?
        .into_iter()
        .map(Identity::from_reader)
        .collect::<capnp::Result<Vec<Identity>>>()?;
      Ok(names)
    }))?;

    Ok(result)
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn list(&self, filter: SecretListFilter) -> SecretStoreResult<SecretList> {
    unimplemented!()
  }

  fn add(&self, secret_version: SecretVersion) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret> {
    unimplemented!()
  }
}
