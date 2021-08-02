use crate::api::{read_option, set_text_list, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status};
use crate::api::{Event, EventHandler, EventSubscription, PasswordGeneratorParam};
use crate::api_capnp::{clipboard_control, event_handler, event_subscription, secrets_store, service};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreResult, SecretsStore};
use crate::service::{ClipboardControl, ServiceResult, StoreConfig, TrustlessService};
use capnp::capability::Promise;
use futures::FutureExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::task::LocalSet;

pub struct RemoteTrustlessService {
  client: service::Client,
  runtime: Rc<RefCell<Runtime>>,
  local_set: Rc<LocalSet>,
}

unsafe impl Send for RemoteTrustlessService {}

unsafe impl Sync for RemoteTrustlessService {}

impl RemoteTrustlessService {
  pub fn new(client: service::Client, runtime: Runtime, local_set: LocalSet) -> RemoteTrustlessService {
    RemoteTrustlessService {
      client,
      runtime: Rc::new(RefCell::new(runtime)),
      local_set: Rc::new(local_set),
    }
  }
}

impl TrustlessService for RemoteTrustlessService {
  fn list_stores(&self) -> ServiceResult<Vec<StoreConfig>> {
    let rt = self.runtime.borrow();
    let request = self.client.list_stores_request();
    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        let store_configs = response?
          .get()?
          .get_store_configs()?
          .into_iter()
          .map(StoreConfig::from_reader)
          .collect::<capnp::Result<Vec<StoreConfig>>>()?;
        Ok(store_configs)
      }),
    )
  }

  fn upsert_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    let rt = self.runtime.borrow();
    let mut request = self.client.upsert_store_config_request();
    store_config.to_builder(request.get().init_store_config())?;

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn delete_store_config(&self, name: &str) -> ServiceResult<()> {
    let rt = self.runtime.borrow();
    let mut request = self.client.delete_store_config_request();
    request.get().set_store_name(&name);

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn open_store(&self, name: &str) -> ServiceResult<Arc<dyn SecretsStore>> {
    let rt = self.runtime.borrow();
    let mut request = self.client.open_store_request();
    request.get().set_store_name(name);
    let store_client: ServiceResult<secrets_store::Client> = self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| Ok(response?.get()?.get_store()?)),
    );

    Ok(Arc::new(RemoteSecretsStore::new(
      store_client?,
      self.runtime.clone(),
      self.local_set.clone(),
    )?))
  }

  fn get_default_store(&self) -> ServiceResult<Option<String>> {
    let rt = self.runtime.borrow();
    let request = self.client.get_default_store_request();
    self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(read_option(response?.get()?.get_store_name()?)?.map(ToString::to_string))),
    )
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    let rt = self.runtime.borrow();
    let mut request = self.client.set_default_store_request();
    request.get().set_store_name(&name);

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn secret_to_clipboard(
    &self,
    store_name: &str,
    block_id: &str,
    properties: &[&str],
    display_name: &str,
  ) -> ServiceResult<Arc<dyn ClipboardControl>> {
    let rt = self.runtime.borrow();
    let mut request = self.client.secret_to_clipboard_request();

    request.get().set_store_name(store_name);
    request.get().set_block_id(block_id);
    set_text_list(request.get().init_properties(properties.len() as u32), properties)?;
    request.get().set_display_name(display_name);

    let clipboard_control_client: ServiceResult<clipboard_control::Client> = self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_clipboard_control()?)),
    );

    Ok(Arc::new(RemoteClipboardControl::new(
      clipboard_control_client?,
      self.runtime.clone(),
      self.local_set.clone(),
    )?))
  }

  fn add_event_handler(&self, handler: Box<dyn EventHandler>) -> ServiceResult<Box<dyn EventSubscription>> {
    let rt = self.runtime.borrow();
    let mut request = self.client.add_event_handler_request();

    request
      .get()
      .set_handler(capnp_rpc::new_client(RemoteEventHandlerImpl::new(handler)));
    let subscription_client: ServiceResult<event_subscription::Client> = self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_subscription()?)),
    );

    Ok(Box::new(RemoteEventSubscription(subscription_client?)))
  }

  fn generate_id(&self) -> ServiceResult<String> {
    let rt = self.runtime.borrow();
    let request = self.client.generate_id_request();

    self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_id()?.to_string())),
    )
  }

  fn generate_password(&self, param: PasswordGeneratorParam) -> ServiceResult<String> {
    let rt = self.runtime.borrow();
    let mut request = self.client.generate_password_request();
    param.to_builder(request.get().init_param())?;

    self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_password()?.to_string())),
    )
  }

  fn check_autolock(&self) {
    // This is done by the daemon itself
  }
}

impl std::fmt::Debug for RemoteTrustlessService {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Remote Trustless service")
  }
}

pub struct RemoteSecretsStore {
  client: secrets_store::Client,
  runtime: Rc<RefCell<Runtime>>,
  local_set: Rc<LocalSet>,
}

unsafe impl Send for RemoteSecretsStore {}

unsafe impl Sync for RemoteSecretsStore {}

impl RemoteSecretsStore {
  fn new(
    client: secrets_store::Client,
    runtime: Rc<RefCell<Runtime>>,
    local_set: Rc<LocalSet>,
  ) -> ServiceResult<RemoteSecretsStore> {
    Ok(RemoteSecretsStore {
      client,
      runtime,
      local_set,
    })
  }
}

impl SecretsStore for RemoteSecretsStore {
  fn status(&self) -> SecretStoreResult<Status> {
    let rt = self.runtime.borrow();
    let request = self.client.status_request();
    self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(Status::from_reader(response?.get()?.get_status()?)?)),
    )
  }

  fn lock(&self) -> SecretStoreResult<()> {
    let rt = self.runtime.borrow();
    let request = self.client.lock_request();

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let rt = self.runtime.borrow();
    let mut request = self.client.unlock_request();
    request.get().set_identity_id(&identity_id);
    request.get().set_passphrase(&passphrase.borrow());

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    let rt = self.runtime.borrow();
    let request = self.client.identities_request();
    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        let names = response?
          .get()?
          .get_identities()?
          .into_iter()
          .map(Identity::from_reader)
          .collect::<capnp::Result<Vec<Identity>>>()?;
        Ok(names)
      }),
    )
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let rt = self.runtime.borrow();
    let mut request = self.client.add_identity_request();

    identity.to_builder(request.get().init_identity())?;
    request.get().set_passphrase(&passphrase.borrow());
    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let rt = self.runtime.borrow();
    let mut request = self.client.change_passphrase_request();
    request.get().set_passphrase(&passphrase.borrow());

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    let rt = self.runtime.borrow();
    let mut request = self.client.list_request();
    filter.to_builder(request.get().init_filter())?;

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        let list = SecretList::from_reader(response?.get()?.get_list()?)?;

        Ok(list)
      }),
    )
  }

  fn update_index(&self) -> SecretStoreResult<()> {
    let rt = self.runtime.borrow();
    let request = self.client.update_index_request();

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn add(&self, secret_version: SecretVersion) -> SecretStoreResult<String> {
    let rt = self.runtime.borrow();
    let mut request = self.client.add_request();

    secret_version.to_builder(request.get().init_version())?;
    self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_block_id()?.to_string())),
    )
  }

  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret> {
    let rt = self.runtime.borrow();
    let mut request = self.client.get_request();
    request.get().set_id(&secret_id);

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        let secret = Secret::from_reader(response?.get()?.get_secret()?)?;

        Ok(secret)
      }),
    )
  }

  fn get_version(&self, block_id: &str) -> SecretStoreResult<SecretVersion> {
    let rt = self.runtime.borrow();
    let mut request = self.client.get_version_request();
    request.get().set_block_id(&block_id);

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        let secret_version = SecretVersion::from_reader(response?.get()?.get_version()?)?;

        Ok(secret_version)
      }),
    )
  }
}

impl std::fmt::Debug for RemoteSecretsStore {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "Remote secrets store")
  }
}

struct RemoteClipboardControl {
  client: clipboard_control::Client,
  runtime: Rc<RefCell<Runtime>>,
  local_set: Rc<LocalSet>,
}

unsafe impl Send for RemoteClipboardControl {}

unsafe impl Sync for RemoteClipboardControl {}

impl RemoteClipboardControl {
  fn new(
    client: clipboard_control::Client,
    runtime: Rc<RefCell<Runtime>>,
    local_set: Rc<LocalSet>,
  ) -> ServiceResult<RemoteClipboardControl> {
    Ok(RemoteClipboardControl {
      client,
      runtime,
      local_set,
    })
  }
}

impl ClipboardControl for RemoteClipboardControl {
  fn is_done(&self) -> ServiceResult<bool> {
    let rt = self.runtime.borrow();
    let request = self.client.is_done_request();

    self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_is_done())),
    )
  }

  fn currently_providing(&self) -> ServiceResult<Option<String>> {
    let rt = self.runtime.borrow();
    let request = self.client.currently_providing_request();

    self.local_set.block_on(
      &rt,
      request
        .send()
        .promise
        .map(|response| match read_option(response?.get()?.get_providing()?)? {
          None => Ok(None),
          Some(name) => Ok(Some(name.to_string())),
        }),
    )
  }

  fn provide_next(&self) -> ServiceResult<()> {
    let rt = self.runtime.borrow();
    let request = self.client.provide_next_request();

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }

  fn destroy(&self) -> ServiceResult<()> {
    let rt = self.runtime.borrow();
    let request = self.client.destroy_request();

    self.local_set.block_on(
      &rt,
      request.send().promise.map(|response| {
        response?.get()?;

        Ok(())
      }),
    )
  }
}

struct RemoteEventSubscription(event_subscription::Client);

impl EventSubscription for RemoteEventSubscription {}

unsafe impl Send for RemoteEventSubscription {}

unsafe impl Sync for RemoteEventSubscription {}

struct RemoteEventHandlerImpl {
  event_handler: Box<dyn EventHandler>,
}

impl RemoteEventHandlerImpl {
  fn new(event_handler: Box<dyn EventHandler>) -> RemoteEventHandlerImpl {
    RemoteEventHandlerImpl { event_handler }
  }
}

impl event_handler::Server for RemoteEventHandlerImpl {
  fn handle(
    &mut self,
    params: event_handler::HandleParams,
    _: event_handler::HandleResults,
  ) -> Promise<(), capnp::Error> {
    let event = match params
      .get()
      .and_then(event_handler::handle_params::Reader::get_event)
      .and_then(Event::from_reader)
    {
      Ok(event) => event,
      Err(err) => return Promise::err(err),
    };
    self.event_handler.handle(event);

    Promise::ok(())
  }
}
