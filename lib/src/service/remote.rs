use crate::api::{read_option, set_text_list, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status};
use crate::api::{Event, EventHandler, EventSubscription};
use crate::api_capnp::{clipboard_control, event_handler, event_subscription, secrets_store, service};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreResult, SecretsStore};
use crate::service::{ClipboardControl, ServiceResult, StoreConfig, TrustlessService};
use capnp::capability::Promise;
use futures::executor::LocalPool;
use futures::FutureExt;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

pub struct RemoteTrustlessService {
  client: service::Client,
  local_pool: Rc<RefCell<LocalPool>>,
}

impl RemoteTrustlessService {
  pub fn new(client: service::Client, local_pool: LocalPool) -> RemoteTrustlessService {
    RemoteTrustlessService {
      client,
      local_pool: Rc::new(RefCell::new(local_pool)),
    }
  }
}

impl TrustlessService for RemoteTrustlessService {
  fn list_stores(&self) -> ServiceResult<Vec<String>> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.list_stores_request();
    local_pool.run_until(request.send().promise.map(|response| {
      let names = response?
        .get()?
        .get_store_names()?
        .into_iter()
        .map(|name| name.map(ToString::to_string))
        .collect::<capnp::Result<Vec<String>>>()?;
      Ok(names)
    }))
  }

  fn set_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.set_store_config_request();
    store_config.to_builder(request.get().init_store_config());

    local_pool.run_until(request.send().promise.map(|response| {
      response?.get()?;

      Ok(())
    }))
  }

  fn get_store_config(&self, name: &str) -> ServiceResult<StoreConfig> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.get_store_config_request();
    request.get().set_store_name(&name);
    local_pool.run_until(request.send().promise.map(|response| {
      let store_config = StoreConfig::from_reader(response?.get()?.get_store_config()?)?;

      Ok(store_config)
    }))
  }

  fn open_store(&self, name: &str) -> ServiceResult<Arc<dyn SecretsStore>> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.open_store_request();
    request.get().set_store_name(name);
    let store_client: ServiceResult<secrets_store::Client> =
      local_pool.run_until(request.send().promise.map(|response| Ok(response?.get()?.get_store()?)));

    Ok(Arc::new(RemoteSecretsStore::new(
      store_client?,
      self.local_pool.clone(),
    )?))
  }

  fn get_default_store(&self) -> ServiceResult<Option<String>> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.get_default_store_request();
    local_pool.run_until(
      request
        .send()
        .promise
        .map(|response| Ok(read_option(response?.get()?.get_store_name()?)?.map(ToString::to_string))),
    )
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.set_default_store_request();
    request.get().set_store_name(&name);

    local_pool.run_until(request.send().promise.map(|response| {
      response?.get()?;

      Ok(())
    }))
  }

  fn secret_to_clipboard(
    &self,
    store_name: &str,
    secret_id: &str,
    properties: &[&str],
    display_name: &str,
  ) -> ServiceResult<Arc<dyn ClipboardControl>> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.secret_to_clipboard_request();

    request.get().set_store_name(store_name);
    request.get().set_secret_id(secret_id);
    set_text_list(request.get().init_properties(properties.len() as u32), properties)?;
    request.get().set_display_name(display_name);

    let clipboard_control_client: ServiceResult<clipboard_control::Client> = local_pool.run_until(
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_clipboard_control()?)),
    );

    Ok(Arc::new(RemoteClipboardControl::new(
      clipboard_control_client?,
      self.local_pool.clone(),
    )?))
  }

  fn add_event_handler(&self, handler: Box<dyn EventHandler>) -> ServiceResult<Box<dyn EventSubscription>> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.add_event_handler_request();

    request.get().set_handler(
      event_handler::ToClient::new(RemoteEventHandlerImpl::new(handler)).into_client::<capnp_rpc::Server>(),
    );
    let subscription_client: ServiceResult<event_subscription::Client> = local_pool.run_until(
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_subscription()?)),
    );

    Ok(Box::new(RemoteEventSubscription(subscription_client?)))
  }
}

pub struct RemoteSecretsStore {
  client: secrets_store::Client,
  local_pool: Rc<RefCell<LocalPool>>,
}

impl RemoteSecretsStore {
  fn new(client: secrets_store::Client, local_pool: Rc<RefCell<LocalPool>>) -> ServiceResult<RemoteSecretsStore> {
    Ok(RemoteSecretsStore { client, local_pool })
  }
}

impl SecretsStore for RemoteSecretsStore {
  fn status(&self) -> SecretStoreResult<Status> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.status_request();
    local_pool.run_until(
      request
        .send()
        .promise
        .map(|response| Ok(Status::from_reader(response?.get()?.get_status()?)?)),
    )
  }

  fn lock(&self) -> SecretStoreResult<()> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.lock_request();

    local_pool.run_until(request.send().promise.map(|response| {
      response?.get()?;

      Ok(())
    }))
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.unlock_request();
    request.get().set_identity_id(&identity_id);
    request.get().set_passphrase(&passphrase.borrow());

    local_pool.run_until(request.send().promise.map(|response| {
      response?.get()?;

      Ok(())
    }))
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.identities_request();
    local_pool.run_until(request.send().promise.map(|response| {
      let names = response?
        .get()?
        .get_identities()?
        .into_iter()
        .map(Identity::from_reader)
        .collect::<capnp::Result<Vec<Identity>>>()?;
      Ok(names)
    }))
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.add_identity_request();

    identity.to_builder(request.get().init_identity());
    request.get().set_passphrase(&passphrase.borrow());
    local_pool.run_until(request.send().promise.map(|response| {
      response?.get()?;

      Ok(())
    }))
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.change_passphrase_request();
    request.get().set_passphrase(&passphrase.borrow());

    local_pool.run_until(request.send().promise.map(|response| {
      response?.get()?;

      Ok(())
    }))
  }

  fn list(&self, filter: SecretListFilter) -> SecretStoreResult<SecretList> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.list_request();
    filter.to_builder(request.get().init_filter())?;

    local_pool.run_until(request.send().promise.map(|response| {
      let list = SecretList::from_reader(response?.get()?.get_list()?)?;

      Ok(list)
    }))
  }

  fn add(&self, secret_version: SecretVersion) -> SecretStoreResult<String> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.add_request();

    secret_version.to_builder(request.get().init_version())?;
    local_pool.run_until(
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_block_id()?.to_string())),
    )
  }

  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.get_request();
    request.get().set_id(&secret_id);

    local_pool.run_until(request.send().promise.map(|response| {
      let secret = Secret::from_reader(response?.get()?.get_secret()?)?;

      Ok(secret)
    }))
  }

  fn get_version(&self, block_id: &str) -> SecretStoreResult<SecretVersion> {
    let mut local_pool = self.local_pool.borrow_mut();
    let mut request = self.client.get_version_request();
    request.get().set_block_id(&block_id);

    local_pool.run_until(request.send().promise.map(|response| {
      let secret_version = SecretVersion::from_reader(response?.get()?.get_version()?)?;

      Ok(secret_version)
    }))
  }
}

struct RemoteClipboardControl {
  client: clipboard_control::Client,
  local_pool: Rc<RefCell<LocalPool>>,
}

impl RemoteClipboardControl {
  fn new(
    client: clipboard_control::Client,
    local_pool: Rc<RefCell<LocalPool>>,
  ) -> ServiceResult<RemoteClipboardControl> {
    Ok(RemoteClipboardControl { client, local_pool })
  }
}

impl ClipboardControl for RemoteClipboardControl {
  fn is_done(&self) -> ServiceResult<bool> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.is_done_request();

    local_pool.run_until(
      request
        .send()
        .promise
        .map(|response| Ok(response?.get()?.get_is_done())),
    )
  }

  fn currently_providing(&self) -> ServiceResult<Option<String>> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.currently_providing_request();

    local_pool.run_until(
      request
        .send()
        .promise
        .map(|response| match read_option(response?.get()?.get_providing()?)? {
          None => Ok(None),
          Some(name) => Ok(Some(name.to_string())),
        }),
    )
  }

  fn destroy(&self) -> ServiceResult<()> {
    let mut local_pool = self.local_pool.borrow_mut();
    let request = self.client.destroy_request();

    local_pool.run_until(request.send().promise.map(|response| {
      response?.get()?;

      Ok(())
    }))
  }
}

struct RemoteEventSubscription(event_subscription::Client);

impl EventSubscription for RemoteEventSubscription {}

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
