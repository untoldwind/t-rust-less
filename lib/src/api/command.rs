use crate::api_capnp::{command, command_result};
use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
use crate::service::{ServiceError, ServiceResult};
use zeroize::Zeroize;

use super::{
  set_text_list, CapnpSerializing, Event, Identity, PasswordGeneratorParam, Secret, SecretList, SecretListFilter,
  SecretVersion, Status, StoreConfig,
};

#[derive(Debug, Zeroize)]
#[allow(clippy::large_enum_variant)]
#[zeroize(drop)]
pub enum Command {
  ListStores,
  UpsertStoreConfig(StoreConfig),
  DeleteStoreConfig(String),
  GetDefaultStore,
  SetDefaultStore(String),
  GenerateId,
  GeneratePassword(PasswordGeneratorParam),
  PollEvents(u64),

  Status(String),
  Lock(String),
  Unlock {
    store_name: String,
    identity_id: String,
    passphrase: SecretBytes,
  },
  Identities(String),
  AddIdentity {
    store_name: String,
    identity: Identity,
    passphrase: SecretBytes,
  },
  ChangePassphrase {
    store_name: String,
    passphrase: SecretBytes,
  },
  List {
    store_name: String,
    filter: SecretListFilter,
  },
  UpdateIndex(String),
  Add {
    store_name: String,
    secret_version: SecretVersion,
  },
  Get {
    store_name: String,
    secret_id: String,
  },
  GetVersion {
    store_name: String,
    block_id: String,
  },

  SecretToClipboard {
    store_name: String,
    block_id: String,
    properties: Vec<String>,
    display_name: String,
  },
  ClipboardIsDone,
  ClipboardCurrentlyProviding,
  ClipboardProvideNext,
  ClipboardDestroy,
}

impl CapnpSerializing for Command {
  type Owned = command::Owned;

  fn from_reader(reader: command::Reader) -> capnp::Result<Self> {
    match reader.which()? {
      command::ListStores(_) => Ok(Command::ListStores),
      command::UpsertStoreConfig(config) => Ok(Command::UpsertStoreConfig(StoreConfig::from_reader(config?)?)),
      command::DeleteStoreConfig(name) => Ok(Command::DeleteStoreConfig(name?.to_string())),
      command::GetDefaultStore(_) => Ok(Command::GetDefaultStore),
      command::SetDefaultStore(name) => Ok(Command::SetDefaultStore(name?.to_string())),
      command::GenerateId(_) => Ok(Command::GenerateId),
      command::GeneratePassword(param) => Ok(Command::GeneratePassword(PasswordGeneratorParam::from_reader(param?)?)),
      command::PollEvents(last_id) => Ok(Command::PollEvents(last_id)),
      command::Status(name) => Ok(Command::Status(name?.to_string())),
      command::Lock(name) => Ok(Command::Lock(name?.to_string())),
      command::Unlock(args) => Ok(Command::Unlock {
        store_name: args.get_store_name()?.to_string(),
        identity_id: args.get_identity_id()?.to_string(),
        passphrase: SecretBytes::from_secured(args.get_passphrase()?),
      }),
      command::Identities(name) => Ok(Command::Identities(name?.to_string())),
      command::AddIdentity(args) => Ok(Command::AddIdentity {
        store_name: args.get_store_name()?.to_string(),
        identity: Identity::from_reader(args.get_identity()?)?,
        passphrase: SecretBytes::from_secured(args.get_passphrase()?),
      }),
      command::ChangePassphrase(args) => Ok(Command::ChangePassphrase {
        store_name: args.get_store_name()?.to_string(),
        passphrase: SecretBytes::from_secured(args.get_passphrase()?),
      }),
      command::List(args) => Ok(Command::List {
        store_name: args.get_store_name()?.to_string(),
        filter: SecretListFilter::from_reader(args.get_filter()?)?,
      }),
      command::UpdateIndex(name) => Ok(Command::UpdateIndex(name?.to_string())),
      command::Add(args) => Ok(Command::Add {
        store_name: args.get_store_name()?.to_string(),
        secret_version: SecretVersion::from_reader(args.get_secret_version()?)?,
      }),
      command::Get(args) => Ok(Command::Get {
        store_name: args.get_store_name()?.to_string(),
        secret_id: args.get_secret_id()?.to_string(),
      }),
      command::GetVersion(args) => Ok(Command::GetVersion {
        store_name: args.get_store_name()?.to_string(),
        block_id: args.get_block_id()?.to_string(),
      }),
      command::SecretToClipboard(args) => Ok(Command::SecretToClipboard {
        store_name: args.get_store_name()?.to_string(),
        block_id: args.get_block_id()?.to_string(),
        properties: args
          .get_properties()?
          .into_iter()
          .map(|t| t.map(|t| t.to_string()))
          .collect::<capnp::Result<Vec<String>>>()?,
        display_name: args.get_display_name()?.to_string(),
      }),
      command::ClipboardIsDone(_) => Ok(Command::ClipboardIsDone),
      command::ClipboardCurrentlyProviding(_) => Ok(Command::ClipboardCurrentlyProviding),
      command::ClipboardProvideNext(_) => Ok(Command::ClipboardProvideNext),
      command::ClipboardDestroy(_) => Ok(Command::ClipboardDestroy),
    }
  }

  fn to_builder(&self, mut builder: command::Builder) -> capnp::Result<()> {
    match self {
      Command::ListStores => builder.set_list_stores(()),
      Command::UpsertStoreConfig(config) => config.to_builder(builder.init_upsert_store_config())?,
      Command::DeleteStoreConfig(name) => builder.set_delete_store_config(name),
      Command::GetDefaultStore => builder.set_get_default_store(()),
      Command::SetDefaultStore(name) => builder.set_set_default_store(name),
      Command::GenerateId => builder.set_generate_id(()),
      Command::GeneratePassword(param) => param.to_builder(builder.init_generate_password())?,
      Command::PollEvents(last_id) => builder.set_poll_events(*last_id),
      Command::Status(name) => builder.set_status(name),
      Command::Lock(name) => builder.set_lock(name),
      Command::Unlock {
        store_name,
        identity_id,
        passphrase,
      } => {
        let mut unlock = builder.init_unlock();
        unlock.set_store_name(store_name);
        unlock.set_identity_id(identity_id);
        unlock.set_passphrase(passphrase.borrow().as_bytes());
      }
      Command::Identities(name) => builder.set_identities(name),
      Command::AddIdentity {
        store_name,
        identity,
        passphrase,
      } => {
        let mut add_identity = builder.init_add_identity();
        add_identity.set_store_name(store_name);
        identity.to_builder(add_identity.reborrow().init_identity())?;
        add_identity.set_passphrase(passphrase.borrow().as_bytes());
      }
      Command::ChangePassphrase { store_name, passphrase } => {
        let mut change_passphrase = builder.init_change_passphrase();
        change_passphrase.set_store_name(store_name);
        change_passphrase.set_passphrase(passphrase.borrow().as_bytes());
      }
      Command::List { store_name, filter } => {
        let mut list = builder.init_list();
        list.set_store_name(store_name);
        filter.to_builder(list.init_filter())?;
      }
      Command::UpdateIndex(name) => builder.set_update_index(name),
      Command::Add {
        store_name,
        secret_version,
      } => {
        let mut add = builder.init_add();
        add.set_store_name(store_name);
        secret_version.to_builder(add.init_secret_version())?;
      }
      Command::Get { store_name, secret_id } => {
        let mut get = builder.init_get();
        get.set_store_name(store_name);
        get.set_secret_id(secret_id);
      }
      Command::GetVersion { store_name, block_id } => {
        let mut get_version = builder.init_get_version();
        get_version.set_store_name(store_name);
        get_version.set_block_id(block_id);
      }
      Command::SecretToClipboard {
        store_name,
        block_id,
        properties,
        display_name,
      } => {
        let mut secret_to_clipboard = builder.init_secret_to_clipboard();
        secret_to_clipboard.set_store_name(store_name);
        secret_to_clipboard.set_block_id(block_id);
        set_text_list(
          secret_to_clipboard.reborrow().init_properties(properties.len() as u32),
          properties,
        )?;
        secret_to_clipboard.set_display_name(display_name);
      }
      Command::ClipboardIsDone => builder.set_clipboard_is_done(()),
      Command::ClipboardCurrentlyProviding => builder.set_clipboard_currently_providing(()),
      Command::ClipboardProvideNext => builder.set_clipboard_provide_next(()),
      Command::ClipboardDestroy => builder.set_clipboard_destroy(()),
    }
    Ok(())
  }
}

#[derive(Debug, Zeroize)]
#[allow(clippy::large_enum_variant)]
#[zeroize(drop)]
pub enum CommandResult {
  Void,
  Bool(bool),
  String(String),
  Configs(Vec<StoreConfig>),
  Events(Vec<Event>),
  Status(Status),
  SecretList(SecretList),
  Identities(Vec<Identity>),
  Secret(Secret),
  SecretVersion(SecretVersion),
  SecretStoreError(SecretStoreError),
  ServiceError(ServiceError),
}

impl CapnpSerializing for CommandResult {
  type Owned = command_result::Owned;

  #[allow(clippy::redundant_closure)]
  fn from_reader(reader: command_result::Reader) -> capnp::Result<Self> {
    match reader.which()? {
      command_result::Void(_) => Ok(CommandResult::Void),
      command_result::Bool(value) => Ok(CommandResult::Bool(value)),
      command_result::String(value) => Ok(CommandResult::String(value?.to_string())),
      command_result::Configs(configs) => Ok(CommandResult::Configs(
        configs?
          .into_iter()
          .map(|e| StoreConfig::from_reader(e))
          .collect::<capnp::Result<Vec<StoreConfig>>>()?,
      )),
      command_result::Events(events) => Ok(CommandResult::Events(
        events?
          .into_iter()
          .map(|e| Event::from_reader(e))
          .collect::<capnp::Result<Vec<Event>>>()?,
      )),
      command_result::Status(status) => Ok(CommandResult::Status(Status::from_reader(status?)?)),
      command_result::SecretList(secret_list) => Ok(CommandResult::SecretList(SecretList::from_reader(secret_list?)?)),
      command_result::Identities(identities) => Ok(CommandResult::Identities(
        identities?
          .into_iter()
          .map(|e| Identity::from_reader(e))
          .collect::<capnp::Result<Vec<Identity>>>()?,
      )),
      command_result::Secret(secret) => Ok(CommandResult::Secret(Secret::from_reader(secret?)?)),
      command_result::SecretVersion(secret_version) => Ok(CommandResult::SecretVersion(SecretVersion::from_reader(
        secret_version?,
      )?)),
      command_result::SecretStoreError(error) => {
        Ok(CommandResult::SecretStoreError(SecretStoreError::from_reader(error?)?))
      }
      command_result::ServiceError(error) => Ok(CommandResult::ServiceError(ServiceError::from_reader(error?)?)),
    }
  }

  fn to_builder(&self, mut builder: command_result::Builder) -> capnp::Result<()> {
    match self {
      CommandResult::Void => builder.set_void(()),
      CommandResult::Bool(value) => builder.set_bool(*value),
      CommandResult::String(value) => builder.set_string(value),
      CommandResult::Configs(configs) => {
        let mut result = builder.init_configs(configs.len() as u32);

        for (idx, store_config) in configs.iter().enumerate() {
          store_config.to_builder(result.reborrow().get(idx as u32))?;
        }
      }
      CommandResult::Events(events) => {
        let mut result = builder.init_events(events.len() as u32);

        for (idx, event) in events.iter().enumerate() {
          event.to_builder(result.reborrow().get(idx as u32))?;
        }
      }
      CommandResult::Status(status) => status.to_builder(builder.init_status())?,
      CommandResult::SecretList(secret_list) => secret_list.to_builder(builder.init_secret_list())?,
      CommandResult::Identities(identities) => {
        let mut result = builder.init_identities(identities.len() as u32);

        for (idx, identity) in identities.iter().enumerate() {
          identity.to_builder(result.reborrow().get(idx as u32))?;
        }
      }
      CommandResult::Secret(secret) => secret.to_builder(builder.init_secret())?,
      CommandResult::SecretVersion(secret_version) => secret_version.to_builder(builder.init_secret_version())?,
      CommandResult::SecretStoreError(error) => error.to_builder(builder.init_secret_store_error())?,
      CommandResult::ServiceError(error) => error.to_builder(builder.init_service_error())?,
    }
    Ok(())
  }
}

impl From<CommandResult> for ServiceResult<()> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Void => Ok(()),
      CommandResult::ServiceError(error) => Err(error),
      CommandResult::SecretStoreError(error) => Err(ServiceError::SecretsStore(error)),
      _ => Err(ServiceError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<ServiceResult<()>> for CommandResult {
  fn from(result: ServiceResult<()>) -> Self {
    match result {
      Ok(_) => CommandResult::Void,
      Err(error) => CommandResult::ServiceError(error),
    }
  }
}

impl From<CommandResult> for ServiceResult<bool> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Bool(value) => Ok(value),
      CommandResult::ServiceError(error) => Err(error),
      CommandResult::SecretStoreError(error) => Err(ServiceError::SecretsStore(error)),
      _ => Err(ServiceError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<ServiceResult<bool>> for CommandResult {
  fn from(result: ServiceResult<bool>) -> Self {
    match result {
      Ok(value) => CommandResult::Bool(value),
      Err(error) => CommandResult::ServiceError(error),
    }
  }
}

impl From<CommandResult> for ServiceResult<String> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::String(value) => Ok(value),
      CommandResult::ServiceError(error) => Err(error),
      CommandResult::SecretStoreError(error) => Err(ServiceError::SecretsStore(error)),
      _ => Err(ServiceError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<ServiceResult<String>> for CommandResult {
  fn from(result: ServiceResult<String>) -> Self {
    match result {
      Ok(value) => CommandResult::String(value),
      Err(error) => CommandResult::ServiceError(error),
    }
  }
}

impl From<CommandResult> for ServiceResult<Option<String>> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Void => Ok(None),
      CommandResult::String(value) => Ok(Some(value)),
      CommandResult::ServiceError(error) => Err(error),
      CommandResult::SecretStoreError(error) => Err(ServiceError::SecretsStore(error)),
      _ => Err(ServiceError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<ServiceResult<Option<String>>> for CommandResult {
  fn from(result: ServiceResult<Option<String>>) -> Self {
    match result {
      Ok(Some(value)) => CommandResult::String(value),
      Ok(None) => CommandResult::Void,
      Err(error) => CommandResult::ServiceError(error),
    }
  }
}

impl From<CommandResult> for ServiceResult<Vec<StoreConfig>> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Configs(value) => Ok(value),
      CommandResult::ServiceError(error) => Err(error),
      CommandResult::SecretStoreError(error) => Err(ServiceError::SecretsStore(error)),
      _ => Err(ServiceError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<ServiceResult<Vec<StoreConfig>>> for CommandResult {
  fn from(result: ServiceResult<Vec<StoreConfig>>) -> Self {
    match result {
      Ok(value) => CommandResult::Configs(value),
      Err(error) => CommandResult::ServiceError(error),
    }
  }
}

impl From<CommandResult> for ServiceResult<Vec<Event>> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Events(value) => Ok(value),
      CommandResult::ServiceError(error) => Err(error),
      CommandResult::SecretStoreError(error) => Err(ServiceError::SecretsStore(error)),
      _ => Err(ServiceError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<ServiceResult<Vec<Event>>> for CommandResult {
  fn from(result: ServiceResult<Vec<Event>>) -> Self {
    match result {
      Ok(value) => CommandResult::Events(value),
      Err(error) => CommandResult::ServiceError(error),
    }
  }
}

impl From<CommandResult> for SecretStoreResult<()> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Void => Ok(()),
      CommandResult::SecretStoreError(error) => Err(error),
      _ => Err(SecretStoreError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<SecretStoreResult<()>> for CommandResult {
  fn from(result: SecretStoreResult<()>) -> Self {
    match result {
      Ok(_) => CommandResult::Void,
      Err(error) => CommandResult::SecretStoreError(error),
    }
  }
}

impl From<CommandResult> for SecretStoreResult<String> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::String(value) => Ok(value),
      CommandResult::SecretStoreError(error) => Err(error),
      _ => Err(SecretStoreError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<SecretStoreResult<String>> for CommandResult {
  fn from(result: SecretStoreResult<String>) -> Self {
    match result {
      Ok(value) => CommandResult::String(value),
      Err(error) => CommandResult::SecretStoreError(error),
    }
  }
}

impl From<CommandResult> for SecretStoreResult<Status> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Status(value) => Ok(value),
      CommandResult::SecretStoreError(error) => Err(error),
      _ => Err(SecretStoreError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<SecretStoreResult<Status>> for CommandResult {
  fn from(result: SecretStoreResult<Status>) -> Self {
    match result {
      Ok(value) => CommandResult::Status(value),
      Err(error) => CommandResult::SecretStoreError(error),
    }
  }
}

impl From<CommandResult> for SecretStoreResult<Vec<Identity>> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Identities(value) => Ok(value),
      CommandResult::SecretStoreError(error) => Err(error),
      _ => Err(SecretStoreError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<SecretStoreResult<Vec<Identity>>> for CommandResult {
  fn from(result: SecretStoreResult<Vec<Identity>>) -> Self {
    match result {
      Ok(value) => CommandResult::Identities(value),
      Err(error) => CommandResult::SecretStoreError(error),
    }
  }
}

impl From<CommandResult> for SecretStoreResult<SecretList> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::SecretList(value) => Ok(value),
      CommandResult::SecretStoreError(error) => Err(error),
      _ => Err(SecretStoreError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<SecretStoreResult<SecretList>> for CommandResult {
  fn from(result: SecretStoreResult<SecretList>) -> Self {
    match result {
      Ok(value) => CommandResult::SecretList(value),
      Err(error) => CommandResult::SecretStoreError(error),
    }
  }
}

impl From<CommandResult> for SecretStoreResult<Secret> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::Secret(value) => Ok(value),
      CommandResult::SecretStoreError(error) => Err(error),
      _ => Err(SecretStoreError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<SecretStoreResult<Secret>> for CommandResult {
  fn from(result: SecretStoreResult<Secret>) -> Self {
    match result {
      Ok(value) => CommandResult::Secret(value),
      Err(error) => CommandResult::SecretStoreError(error),
    }
  }
}

impl From<CommandResult> for SecretStoreResult<SecretVersion> {
  fn from(result: CommandResult) -> Self {
    match result {
      CommandResult::SecretVersion(value) => Ok(value),
      CommandResult::SecretStoreError(error) => Err(error),
      _ => Err(SecretStoreError::IO("Invalid command result".to_string())),
    }
  }
}

impl From<SecretStoreResult<SecretVersion>> for CommandResult {
  fn from(result: SecretStoreResult<SecretVersion>) -> Self {
    match result {
      Ok(value) => CommandResult::SecretVersion(value),
      Err(error) => CommandResult::SecretStoreError(error),
    }
  }
}
