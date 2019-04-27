use crate::api::{Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreResult, SecretsStore};
use crate::service::{ServiceResult, StoreConfig, TrustlessService};
use crate::service_capnp::{identity, option, secrets_store, service};
use futures::Future;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::runtime::current_thread;
use chrono::Utc;
use chrono::offset::TimeZone;

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
      let status = response.get()?.get_status()?;

      Ok(Status {
        locked: status.get_locked(),
        unlocked_by: read_option(status.get_unlocked_by()?)?.map(read_identity).transpose()?,
        autolock_at: {
          let autolock_at = status.get_autolock_at();
          if autolock_at == std::i64::MIN {
            None
          } else {
            Some(Utc.timestamp_millis(autolock_at))
          }
        },
        version: status.get_version()?.to_string(),
      })
    }))?;

    Ok(result)
  }

  fn lock(&self) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    unimplemented!()
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    unimplemented!()
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

fn read_option<T>(reader: option::Reader<T>) -> capnp::Result<Option<<T as capnp::traits::Owned<'_>>::Reader>>
where
  T: for<'c> capnp::traits::Owned<'c>,
{
  match reader.which()? {
    option::Some(inner) => Ok(Some(inner?)),
    option::None(_) => Ok(None),
  }
}

fn read_identity(reader: identity::Reader) -> capnp::Result<Identity> {
  Ok(Identity {
    id: reader.get_id()?.to_string(),
    name: reader.get_name()?.to_string(),
    email: reader.get_email()?.to_string(),
  })
}
