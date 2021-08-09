use crate::memguard::SecretBytes;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
use crate::service::{ServiceError, ServiceResult};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::{
  Event, Identity, PasswordGeneratorParam, Secret, SecretList, SecretListFilter, SecretVersion, Status, StoreConfig,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
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

#[derive(Debug, Serialize, Deserialize, Zeroize)]
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
