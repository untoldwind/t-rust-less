use crate::api_capnp::{command, result_events, result_identities, result_option_string, result_store_configs};
use crate::memguard::SecretBytes;
use zeroize::Zeroize;

use super::{
  read_option, set_text_list, CapnpSerializing, Event, Identity, PasswordGeneratorParam, SecretListFilter,
  SecretVersion, StoreConfig,
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

pub struct ResultStoreConfigs(pub Vec<StoreConfig>);

impl CapnpSerializing for ResultStoreConfigs {
  type Owned = result_store_configs::Owned;

  #[allow(clippy::redundant_closure)]
  fn from_reader(reader: result_store_configs::Reader) -> capnp::Result<ResultStoreConfigs> {
    Ok(ResultStoreConfigs(
      reader
        .get_configs()?
        .into_iter()
        .map(|c| StoreConfig::from_reader(c))
        .collect::<capnp::Result<Vec<StoreConfig>>>()?,
    ))
  }

  fn to_builder(&self, builder: result_store_configs::Builder) -> capnp::Result<()> {
    let mut result = builder.init_configs(self.0.len() as u32);

    for (idx, store_config) in self.0.iter().enumerate() {
      store_config.to_builder(result.reborrow().get(idx as u32))?;
    }

    Ok(())
  }
}

pub struct ResultOptionString(pub Option<String>);

impl CapnpSerializing for ResultOptionString {
  type Owned = result_option_string::Owned;

  fn from_reader(reader: result_option_string::Reader) -> capnp::Result<ResultOptionString> {
    Ok(ResultOptionString(
      read_option(reader.get_content()?)?.map(ToString::to_string),
    ))
  }

  fn to_builder(&self, builder: result_option_string::Builder) -> capnp::Result<()> {
    match &self.0 {
      Some(content) => builder
        .init_content()
        .set_some(capnp::text::new_reader(content.as_bytes())?)?,
      None => builder.init_content().set_none(()),
    }
    Ok(())
  }
}

pub struct ResultIdentities(pub Vec<Identity>);

impl CapnpSerializing for ResultIdentities {
  type Owned = result_identities::Owned;

  #[allow(clippy::redundant_closure)]
  fn from_reader(reader: result_identities::Reader) -> capnp::Result<ResultIdentities> {
    Ok(ResultIdentities(
      reader
        .get_identities()?
        .into_iter()
        .map(|i| Identity::from_reader(i))
        .collect::<capnp::Result<Vec<Identity>>>()?,
    ))
  }

  fn to_builder(&self, builder: result_identities::Builder) -> capnp::Result<()> {
    let mut result = builder.init_identities(self.0.len() as u32);

    for (idx, store_config) in self.0.iter().enumerate() {
      store_config.to_builder(result.reborrow().get(idx as u32))?;
    }

    Ok(())
  }
}

pub struct ResultEvents(pub Vec<Event>);

impl CapnpSerializing for ResultEvents {
  type Owned = result_events::Owned;

  #[allow(clippy::redundant_closure)]
  fn from_reader(reader: result_events::Reader) -> capnp::Result<ResultEvents> {
    Ok(ResultEvents(
      reader
        .get_events()?
        .into_iter()
        .map(|e| Event::from_reader(e))
        .collect::<capnp::Result<Vec<Event>>>()?,
    ))
  }

  fn to_builder(&self, builder: result_events::Builder) -> capnp::Result<()> {
    let mut result = builder.init_events(self.0.len() as u32);

    for (idx, store_config) in self.0.iter().enumerate() {
      store_config.to_builder(result.reborrow().get(idx as u32))?;
    }

    Ok(())
  }
}
