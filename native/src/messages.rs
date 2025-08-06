use serde::{Deserialize, Serialize};
use t_rust_less_lib::api::{
  ClipboardProviding, Event, Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status, StoreConfig,
};
use t_rust_less_lib::secrets_store::SecretStoreResult;
use t_rust_less_lib::service::{ServiceError, ServiceResult};
use zeroize::Zeroize;

#[derive(Debug, Serialize, Deserialize, Zeroize)]
#[allow(clippy::large_enum_variant)]
#[serde(rename_all = "snake_case")]
pub enum Command {
  ListStores,
  UpsertStoreConfig(StoreConfig),
  DeleteStoreConfig(String),
  GetDefaultStore,
  SetDefaultStore(String),
  DirectClipboardAvailable,
  SecretToClipboard {
    store_name: String,
    block_id: String,
    properties: Vec<String>,
  },

  Status {
    store_name: String,
  },
  Lock {
    store_name: String,
  },
  Unlock {
    store_name: String,
    identity_id: String,
    passphrase: String,
  },

  ListIdentities {
    store_name: String,
  },
  AddIdentity {
    store_name: String,
    identity: Identity,
    passphrase: String,
  },
  ChangePassphrase {
    store_name: String,
    passphrase: String,
  },

  ListSecrets {
    store_name: String,
    filter: SecretListFilter,
  },
  AddSecret {
    store_name: String,
    version: SecretVersion,
  },
  GetSecret {
    store_name: String,
    secret_id: String,
  },
  GetSecretVersion {
    store_name: String,
    block_id: String,
  },

  ClipboardIsDone,
  ClipboardCurrentlyProviding,
  ClipboardDestroy,
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
#[serde(rename_all = "snake_case")]
pub enum CommandResult {
  Invalid,
  Success,
  Error { error: ServiceError, display: String },
  Empty,
  Bool(bool),
  StoreConfigList(Vec<StoreConfig>),
  StoreConfig(StoreConfig),
  String(String),
  StringList(Vec<String>),

  Status(Status),
  Identities(Vec<Identity>),

  SecretList(SecretList),
  SecretVersion(SecretVersion),
  Secret(Secret),

  ClipboardProviding(ClipboardProviding),
}

impl<T> From<ServiceResult<T>> for CommandResult
where
  T: Into<CommandResult>,
{
  fn from(result: ServiceResult<T>) -> Self {
    match result {
      Ok(success) => success.into(),
      Err(error) => {
        let display = format!("{error}");
        CommandResult::Error { error, display }
      }
    }
  }
}

impl<T> From<SecretStoreResult<T>> for CommandResult
where
  T: Into<CommandResult>,
{
  fn from(result: SecretStoreResult<T>) -> Self {
    match result {
      Ok(success) => success.into(),
      Err(error) => {
        let display = format!("{error}");
        CommandResult::Error {
          error: ServiceError::SecretsStore(error),
          display,
        }
      }
    }
  }
}

impl<T> From<Option<T>> for CommandResult
where
  T: Into<CommandResult>,
{
  fn from(maybe: Option<T>) -> Self {
    match maybe {
      Some(value) => value.into(),
      None => CommandResult::Empty,
    }
  }
}

impl From<()> for CommandResult {
  fn from(_: ()) -> Self {
    CommandResult::Success
  }
}

impl From<bool> for CommandResult {
  fn from(b: bool) -> Self {
    CommandResult::Bool(b)
  }
}

impl From<String> for CommandResult {
  fn from(s: String) -> Self {
    CommandResult::String(s)
  }
}

impl From<Vec<String>> for CommandResult {
  fn from(list: Vec<String>) -> Self {
    CommandResult::StringList(list)
  }
}

impl From<StoreConfig> for CommandResult {
  fn from(config: StoreConfig) -> Self {
    CommandResult::StoreConfig(config)
  }
}

impl From<Vec<StoreConfig>> for CommandResult {
  fn from(configs: Vec<StoreConfig>) -> Self {
    CommandResult::StoreConfigList(configs)
  }
}

impl From<Status> for CommandResult {
  fn from(status: Status) -> Self {
    CommandResult::Status(status)
  }
}

impl From<Vec<Identity>> for CommandResult {
  fn from(list: Vec<Identity>) -> Self {
    CommandResult::Identities(list)
  }
}

impl From<SecretList> for CommandResult {
  fn from(list: SecretList) -> Self {
    CommandResult::SecretList(list)
  }
}

impl From<Secret> for CommandResult {
  fn from(secret: Secret) -> Self {
    CommandResult::Secret(secret)
  }
}

impl From<SecretVersion> for CommandResult {
  fn from(secret: SecretVersion) -> Self {
    CommandResult::SecretVersion(secret)
  }
}

impl From<ClipboardProviding> for CommandResult {
  fn from(clipboard_providing: ClipboardProviding) -> Self {
    CommandResult::ClipboardProviding(clipboard_providing)
  }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
  pub id: u64,
  pub command: Command,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Response {
  Command { id: u64, result: CommandResult },
  Event(Event),
}

#[cfg(test)]
mod tests {
  use super::*;
  use spectral::prelude::*;

  #[test]
  fn test_serialize() {
    let request1 = Request {
      id: 12,
      command: Command::ListStores,
    };
    let request2 = Request {
      id: 13,
      command: Command::Status {
        store_name: "bla".to_string(),
      },
    };

    assert_that(&serde_json::to_string(&request1).unwrap())
      .is_equal_to(r#"{"id":12,"command":"list_stores"}"#.to_string());
    assert_that(&serde_json::to_string(&request2).unwrap())
      .is_equal_to(r#"{"id":13,"command":{"status":{"store_name":"bla"}}}"#.to_string());
  }
}
