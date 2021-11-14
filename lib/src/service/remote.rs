use crate::api::{
  ClipboardProviding, Command, CommandResult, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status,
  StoreConfig,
};
use crate::api::{Event, PasswordGeneratorParam};
use crate::memguard::{SecretBytes, ZeroizeBytesBuffer};
use crate::secrets_store::{SecretStoreError, SecretStoreResult, SecretsStore};
use crate::service::{ClipboardControl, ServiceError, ServiceResult, TrustlessService};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex, MutexGuard};
use zeroize::Zeroizing;

fn write_command<S, E>(writer: &mut MutexGuard<S>, command: Command) -> Result<(), E>
where
  S: Write,
  E: From<std::io::Error> + From<rmp_serde::encode::Error>,
{
  let mut message = ZeroizeBytesBuffer::with_capacity(1024);
  rmp_serde::encode::write_named(&mut message, &command)?;

  writer.write_u32::<LittleEndian>(message.len() as u32)?;
  writer.write_all(&message)?;

  Ok(())
}

fn recv_result<S, E>(reader: &mut MutexGuard<S>) -> Result<CommandResult, E>
where
  S: Read,
  E: From<std::io::Error> + From<rmp_serde::decode::Error>,
{
  let len = reader.read_u32::<LittleEndian>()? as usize;
  let mut buf = Zeroizing::from(vec![0; len]);

  reader.read_exact(&mut buf)?;

  Ok(rmp_serde::from_read_ref(buf.as_slice())?)
}

fn send_recv<'a, S, E>(stream: &'a Arc<Mutex<S>>, command: Command) -> Result<CommandResult, E>
where
  S: Read + Write + 'a,
  E: From<std::io::Error>
    + From<rmp_serde::encode::Error>
    + From<rmp_serde::decode::Error>
    + From<std::sync::PoisonError<std::sync::MutexGuard<'a, S>>>
    + DeserializeOwned,
{
  let mut stream = stream.lock()?;

  write_command::<S, E>(&mut stream, command)?;

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
    send_recv::<_, ServiceError>(&self.stream, Command::ListStores)?.into()
  }

  fn upsert_store_config(&self, store_config: StoreConfig) -> ServiceResult<()> {
    send_recv::<_, ServiceError>(&self.stream, Command::UpsertStoreConfig(store_config))?.into()
  }

  fn delete_store_config(&self, name: &str) -> ServiceResult<()> {
    send_recv::<_, ServiceError>(&self.stream, Command::DeleteStoreConfig(name.to_string()))?.into()
  }

  fn open_store(&self, name: &str) -> SecretStoreResult<Arc<dyn SecretsStore>> {
    Ok(Arc::new(RemoteSecretsStore::new(&self.stream, name)))
  }

  fn get_default_store(&self) -> ServiceResult<Option<String>> {
    send_recv::<_, ServiceError>(&self.stream, Command::GetDefaultStore)?.into()
  }

  fn set_default_store(&self, name: &str) -> ServiceResult<()> {
    send_recv::<_, ServiceError>(&self.stream, Command::SetDefaultStore(name.to_string()))?.into()
  }

  fn secret_to_clipboard(
    &self,
    store_name: &str,
    block_id: &str,
    properties: &[&str],
    display_name: &str,
  ) -> ServiceResult<Arc<dyn ClipboardControl>> {
    send_recv::<_, ServiceError>(
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
    send_recv::<_, ServiceError>(&self.stream, Command::PollEvents(last_id))?.into()
  }

  fn generate_id(&self) -> ServiceResult<String> {
    send_recv::<_, ServiceError>(&self.stream, Command::GenerateId)?.into()
  }

  fn generate_password(&self, param: PasswordGeneratorParam) -> ServiceResult<String> {
    send_recv::<_, ServiceError>(&self.stream, Command::GeneratePassword(param))?.into()
  }

  fn check_autolock(&self) {
    // This should be done by the remote sever itself
  }

  fn synchronize(&self) -> ServiceResult<()> {
    // This should be done by the remote sever itself
    Ok(())
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
    send_recv::<_, SecretStoreError>(&self.stream, Command::Status(self.name.clone()))?.into()
  }

  fn lock(&self) -> SecretStoreResult<()> {
    send_recv::<_, SecretStoreError>(&self.stream, Command::Lock(self.name.clone()))?.into()
  }

  fn unlock(&self, identity_id: &str, passphrase: SecretBytes) -> SecretStoreResult<()> {
    send_recv::<_, SecretStoreError>(
      &self.stream,
      Command::Unlock {
        store_name: self.name.clone(),
        identity_id: identity_id.to_string(),
        passphrase,
      },
    )?
    .into()
  }

  fn identities(&self) -> SecretStoreResult<Vec<Identity>> {
    send_recv::<_, SecretStoreError>(&self.stream, Command::Identities(self.name.clone()))?.into()
  }

  fn add_identity(&self, identity: Identity, passphrase: SecretBytes) -> SecretStoreResult<()> {
    send_recv::<_, SecretStoreError>(
      &self.stream,
      Command::AddIdentity {
        store_name: self.name.clone(),
        identity,
        passphrase,
      },
    )?
    .into()
  }

  fn change_passphrase(&self, passphrase: SecretBytes) -> SecretStoreResult<()> {
    send_recv::<_, SecretStoreError>(
      &self.stream,
      Command::ChangePassphrase {
        store_name: self.name.clone(),
        passphrase,
      },
    )?
    .into()
  }

  fn list(&self, filter: &SecretListFilter) -> SecretStoreResult<SecretList> {
    send_recv::<_, SecretStoreError>(
      &self.stream,
      Command::List {
        store_name: self.name.clone(),
        filter: filter.clone(),
      },
    )?
    .into()
  }

  fn update_index(&self) -> SecretStoreResult<()> {
    send_recv::<_, SecretStoreError>(&self.stream, Command::UpdateIndex(self.name.clone()))?.into()
  }

  fn add(&self, secret_version: SecretVersion) -> SecretStoreResult<String> {
    send_recv::<_, SecretStoreError>(
      &self.stream,
      Command::Add {
        store_name: self.name.clone(),
        secret_version,
      },
    )?
    .into()
  }

  fn get(&self, secret_id: &str) -> SecretStoreResult<Secret> {
    send_recv::<_, SecretStoreError>(
      &self.stream,
      Command::Get {
        store_name: self.name.clone(),
        secret_id: secret_id.to_string(),
      },
    )?
    .into()
  }

  fn get_version(&self, block_id: &str) -> SecretStoreResult<SecretVersion> {
    send_recv::<_, SecretStoreError>(
      &self.stream,
      Command::GetVersion {
        store_name: self.name.clone(),
        block_id: block_id.to_string(),
      },
    )?
    .into()
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
    send_recv::<_, ServiceError>(&self.stream, Command::ClipboardIsDone)?.into()
  }

  fn currently_providing(&self) -> ServiceResult<Option<ClipboardProviding>> {
    send_recv::<_, ServiceError>(&self.stream, Command::ClipboardCurrentlyProviding)?.into()
  }

  fn provide_next(&self) -> ServiceResult<()> {
    send_recv::<_, ServiceError>(&self.stream, Command::ClipboardProvideNext)?.into()
  }

  fn destroy(&self) -> ServiceResult<()> {
    send_recv::<_, ServiceError>(&self.stream, Command::ClipboardDestroy)?.into()
  }
}
