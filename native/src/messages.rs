use serde_derive::{Deserialize, Serialize};
use t_rust_less_lib::api::{Identity, Secret, SecretList, SecretListFilter, SecretVersion, Status};
use t_rust_less_lib::memguard::weak::ZeroingBytes;
use t_rust_less_lib::service::{ServiceError, ServiceResult, StoreConfig};

#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum Command {
  ListStores,
  GetStoreConfig(String),
  SetStoreConfig(StoreConfig),
  GetDefaultStore,
  SetDefaultStore(String),
  DirectClipboardAvailable,
  SecretToClipboard {
    store_name: String,
    secret_id: String,
    properties: Vec<String>,
    display_name: String,
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
    passphrase: ZeroingBytes,
  },

  ListIdentities {
    store_name: String,
  },
  AddIdentity {
    store_name: String,
    identity: Identity,
    passphrase: ZeroingBytes,
  },
  ChangePassphrase {
    store_name: String,
    passphrase: ZeroingBytes,
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
}

#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
pub enum CommandResult {
  Invalid,
  Success,
  Error { error: ServiceError, display: String },
  Empty,
  Bool(bool),
  StoreConfig(StoreConfig),
  String(String),
  StringList(Vec<String>),

  Status(Status),
  Identities(Vec<Identity>),

  SecretList(SecretList),
  SecretVersion(SecretVersion),
  Secret(Secret),
}

impl<T> From<ServiceResult<T>> for CommandResult
where
  T: Into<CommandResult>,
{
  fn from(result: ServiceResult<T>) -> Self {
    match result {
      Ok(success) => success.into(),
      Err(error) => {
        let display = format!("{}", error);
        CommandResult::Error { error, display }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
  pub id: u64,
  pub comamnd: Command,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
  pub id: u64,
  pub result: CommandResult,
}
