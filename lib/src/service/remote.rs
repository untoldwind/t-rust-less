use crate::api::{read_option, set_text_list, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status};
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
        .map(|name| name.map(ToString::to_string))
        .collect::<capnp::Result<Vec<String>>>()?;
      Ok(names)
    }))?;

    Ok(result)
  }

  fn set_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.set_store_config_request();
    store_config.to_builder(request.get().init_store_config());

    runtime.block_on(request.send().promise.and_then(|response| {
      response.get()?;

      Ok(())
    }))?;

    Ok(())
  }

  fn get_store_config(&self, name: &str) -> ServiceResult<StoreConfig> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.get_store_config_request();
    request.get().set_store_name(&name);
    let result = runtime.block_on(request.send().promise.and_then(|response| {
      let store_config = StoreConfig::from_reader(response.get()?.get_store_config()?)?;

      Ok(store_config)
    }))?;

    Ok(result)
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
        .and_then(|response| Ok(read_option(response.get()?.get_store_name()?)?.map(ToString::to_string))),
    )?;

    Ok(result)
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.set_default_store_request();
    request.get().set_store_name(&name);

    runtime.block_on(request.send().promise.and_then(|response| {
      response.get()?;

      Ok(())
    }))?;

    Ok(())
  }

  fn direct_clipboard_available(&self) -> ServiceResult<bool> {
    let mut runtime = self.runtime.borrow_mut();
    let request = self.client.direct_clipboard_available_request();

    let result = runtime.block_on(
      request
        .send()
        .promise
        .and_then(|response| Ok(response.get()?.get_available())),
    )?;

    Ok(result)
  }

  fn secret_to_clipboard(&self, store_name: &str, secret_id: &str, properties: &[&str]) -> ServiceResult<()> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.secret_to_clipboard_request();

    request.get().set_store_name(store_name);
    request.get().set_secret_id(secret_id);
    set_text_list(request.get().init_properties(properties.len() as u32), properties)?;

    runtime.block_on(request.send().promise.and_then(|response| {
      response.get()?;

      Ok(())
    }))?;

    Ok(())
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
    let result = runtime.block_on(
      request
        .send()
        .promise
        .and_then(|response| Ok(Status::from_reader(response.get()?.get_status()?)?)),
    )?;

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
    request.get().set_identity_id(&identity_id);
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
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.add_identity_request();

    identity.to_builder(request.get().init_identity());
    request.get().set_passphrase(&passphrase.borrow());
    runtime.block_on(request.send().promise.and_then(|response| {
      response.get()?;

      Ok(())
    }))?;

    Ok(())
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.change_passphrase_request();
    request.get().set_passphrase(&passphrase.borrow());

    runtime.block_on(request.send().promise.and_then(|response| {
      response.get()?;

      Ok(())
    }))?;

    Ok(())
  }

  fn list(&self, filter: SecretListFilter) -> SecretStoreResult<SecretList> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.list_request();
    filter.to_builder(request.get().init_filter())?;

    let result = runtime.block_on(request.send().promise.and_then(|response| {
      let list = SecretList::from_reader(response.get()?.get_list()?)?;

      Ok(list)
    }))?;

    Ok(result)
  }

  fn add(&self, secret_version: SecretVersion) -> SecretStoreResult<String> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.add_request();

    secret_version.to_builder(request.get().init_version())?;
    let result = runtime.block_on(
      request
        .send()
        .promise
        .and_then(|response| Ok(response.get()?.get_block_id()?.to_string())),
    )?;

    Ok(result)
  }

  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.get_request();
    request.get().set_id(&secret_id);

    let result = runtime.block_on(request.send().promise.and_then(|response| {
      let secret = Secret::from_reader(response.get()?.get_secret()?)?;

      Ok(secret)
    }))?;

    Ok(result)
  }

  fn get_version(&self, block_id: &str) -> SecretStoreResult<SecretVersion> {
    let mut runtime = self.runtime.borrow_mut();
    let mut request = self.client.get_version_request();
    request.get().set_block_id(&block_id);

    let result = runtime.block_on(request.send().promise.and_then(|response| {
      let secret_version = SecretVersion::from_reader(response.get()?.get_version()?)?;

      Ok(secret_version)
    }))?;

    Ok(result)
  }
}
