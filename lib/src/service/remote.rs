use crate::api::{
  CapnpSerializable, Command, Identity, ResultEvents, ResultIdentities, ResultOptionString, ResultStoreConfigs, Secret,
  SecretList, SecretListFilter, SecretVersion, Status, StoreConfig,
};
use crate::api::{Event, PasswordGeneratorParam};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreError, SecretStoreResult, SecretsStore};
use crate::service::{ClipboardControl, ServiceError, ServiceResult, TrustlessService};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use zeroize::Zeroizing;

fn write_command<S, C, E>(writer: &mut MutexGuard<S>, command: C) -> Result<(), E>
where
  S: Write,
  C: CapnpSerializable,
  E: From<std::io::Error> + From<capnp::Error>,
{
  let message = command.serialize_capnp()?;

  writer.write_u32::<LittleEndian>(message.len() as u32)?;
  writer.write_all(&message)?;

  Ok(())
}

fn recv_result<S, R, E>(reader: &mut MutexGuard<S>) -> Result<R, E>
where
  S: Read,
  R: CapnpSerializable,
  E: From<std::io::Error> + From<capnp::Error> + From<serde_json::Error> + DeserializeOwned,
{
  let len = reader.read_u32::<LittleEndian>()? as usize;
  let success = reader.read_u8()?;
  let mut buf = Zeroizing::from(vec![0; len - 1]);

  reader.read_exact(&mut buf)?;

  if success == 0 {
    Err(serde_json::from_reader(buf.as_slice())?)
  } else {
    Ok(R::deserialize_capnp(buf.as_slice())?)
  }
}

fn send_recv<'a, S, C, R, E>(stream: &'a Arc<Mutex<S>>, command: C) -> Result<R, E>
where
  S: Read + Write + 'a,
  C: CapnpSerializable,
  R: CapnpSerializable,
  E: From<std::io::Error>
    + From<capnp::Error>
    + From<serde_json::Error>
    + From<std::sync::PoisonError<std::sync::MutexGuard<'a, S>>>
    + DeserializeOwned,
{
  let mut stream = stream.lock()?;

  write_command::<S, C, E>(&mut stream, command)?;

  recv_result(&mut stream)
}

#[derive(Debug)]
pub struct RemoteTrustlessService<S> {
  stream: Arc<Mutex<S>>,
}

impl<S> RemoteTrustlessService<S>
where
  S: Read + Write + Debug + Send + Sync,
{
  pub fn new(stream: S) -> Self {
    RemoteTrustlessService {
      stream: Arc::new(Mutex::new(stream)),
    }
  }
}

impl<S> TrustlessService for RemoteTrustlessService<S>
where
  S: Read + Write + Debug + Send + Sync + 'static,
{
  fn list_stores(&self) -> ServiceResult<Vec<StoreConfig>> {
    Ok(send_recv::<_, _, ResultStoreConfigs, ServiceError>(&self.stream, Command::ListStores)?.0)
  }

  fn upsert_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    send_recv(&self.stream, Command::UpsertStoreConfig(store_config))
  }

  fn delete_store_config(&self, name: &str) -> ServiceResult<()> {
    send_recv(&self.stream, Command::DeleteStoreConfig(name.to_string()))
  }

  fn open_store(&self, name: &str) -> SecretStoreResult<Arc<dyn SecretsStore>> {
    Ok(Arc::new(RemoteSecretsStore::new(&self.stream, name)))
  }

  fn get_default_store(&self) -> ServiceResult<Option<String>> {
    Ok(send_recv::<_, _, ResultOptionString, ServiceError>(&self.stream, Command::GetDefaultStore)?.0)
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    send_recv(&self.stream, Command::SetDefaultStore(name.to_string()))
  }

  fn secret_to_clipboard(
    &self,
    store_name: &str,
    block_id: &str,
    properties: &[&str],
    display_name: &str,
  ) -> ServiceResult<Arc<dyn ClipboardControl>> {
    send_recv::<_, _, (), ServiceError>(
      &self.stream,
      Command::SecretToClipboard {
        store_name: store_name.to_string(),
        block_id: block_id.to_string(),
        properties: properties.iter().map(ToString::to_string).collect(),
        display_name: display_name.to_string(),
      },
    )?;
    Ok(Arc::new(RemoteClipboardControl::new(&self.stream)))
  }

  fn poll_events(&self, last_id: u64) -> ServiceResult<Vec<Event>> {
    Ok(send_recv::<_, _, ResultEvents, ServiceError>(&self.stream, Command::PollEvents(last_id))?.0)
  }

  fn generate_id(&self) -> ServiceResult<String> {
    send_recv(&self.stream, Command::GenerateId)
  }

  fn generate_password(&self, param: PasswordGeneratorParam) -> ServiceResult<String> {
    send_recv(&self.stream, Command::GeneratePassword(param))
  }

  fn check_autolock(&self) {
    // This should be done by the remote sever itself
  }
}

#[derive(Debug)]
struct RemoteSecretsStore<S> {
  stream: Arc<Mutex<S>>,
  name: String,
}

impl<S> RemoteSecretsStore<S>
where
  S: Read + Write + Debug + Send + Sync,
{
  fn new(stream: &Arc<Mutex<S>>, name: &str) -> Self {
    RemoteSecretsStore {
      stream: stream.clone(),
      name: name.to_string(),
    }
  }
}

impl<S> SecretsStore for RemoteSecretsStore<S>
where
  S: Read + Write + Debug + Send + Sync,
{
  fn status(&self) -> SecretStoreResult<Status> {
    send_recv(&self.stream, Command::Status(self.name.clone()))
  }

  fn lock(&self) -> SecretStoreResult<()> {
    send_recv(&self.stream, Command::Lock(self.name.clone()))
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    send_recv(
      &self.stream,
      Command::Unlock {
        store_name: self.name.clone(),
        identity_id: identity_id.to_string(),
        passphrase,
      },
    )
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    Ok(send_recv::<_, _, ResultIdentities, SecretStoreError>(&self.stream, Command::Identities(self.name.clone()))?.0)
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    send_recv(
      &self.stream,
      Command::AddIdentity {
        store_name: self.name.clone(),
        identity,
        passphrase,
      },
    )
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    send_recv(
      &self.stream,
      Command::ChangePassphrase {
        store_name: self.name.clone(),
        passphrase,
      },
    )
  }

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    send_recv(
      &self.stream,
      Command::List {
        store_name: self.name.clone(),
        filter: filter.clone(),
      },
    )
  }

  fn update_index(&self) -> SecretStoreResult<()> {
    send_recv(&self.stream, Command::UpdateIndex(self.name.clone()))
  }

  fn add(&self, secret_version: SecretVersion) -> SecretStoreResult<String> {
    send_recv(
      &self.stream,
      Command::Add {
        store_name: self.name.clone(),
        secret_version,
      },
    )
  }

  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret> {
    send_recv(
      &self.stream,
      Command::Get {
        store_name: self.name.clone(),
        secret_id: secret_id.to_string(),
      },
    )
  }

  fn get_version(&self, block_id: &str) -> SecretStoreResult<SecretVersion> {
    send_recv(
      &self.stream,
      Command::GetVersion {
        store_name: self.name.clone(),
        block_id: block_id.to_string(),
      },
    )
  }
}

#[derive(Debug)]
struct RemoteClipboardControl<S> {
  stream: Arc<Mutex<S>>,
}

impl<S> RemoteClipboardControl<S>
where
  S: Read + Write + Debug + Send + Sync,
{
  fn new(stream: &Arc<Mutex<S>>) -> Self {
    RemoteClipboardControl { stream: stream.clone() }
  }
}

impl<S> ClipboardControl for RemoteClipboardControl<S>
where
  S: Read + Write + Debug + Send + Sync,
{
  fn is_done(&self) -> ServiceResult<bool> {
    send_recv(&self.stream, Command::ClipboardIsDone)
  }

  fn currently_providing(&self) -> ServiceResult<Option<String>> {
    Ok(send_recv::<_, _, ResultOptionString, ServiceError>(&self.stream, Command::ClipboardCurrentlyProviding)?.0)
  }

  fn provide_next(&self) -> ServiceResult<()> {
    send_recv(&self.stream, Command::ClipboardProvideNext)
  }

  fn destroy(&self) -> ServiceResult<()> {
    send_recv(&self.stream, Command::ClipboardDestroy)
  }
}
