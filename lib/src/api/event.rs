use super::CapnpSerializing;
use crate::api::Identity;
use crate::api_capnp::{event, event_data};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Clone, Debug, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub enum EventData {
  StoreUnlocked {
    store_name: String,
    identity: Identity,
  },
  StoreLocked {
    store_name: String,
  },
  SecretOpened {
    store_name: String,
    identity: Identity,
    secret_id: String,
  },
  SecretVersionAdded {
    store_name: String,
    identity: Identity,
    secret_id: String,
  },
  IdentityAdded {
    store_name: String,
    identity: Identity,
  },
  ClipboardProviding {
    store_name: String,
    block_id: String,
    property: String,
  },
  ClipboardDone,
}

impl CapnpSerializing for EventData {
  type Owned = event_data::Owned;

  fn from_reader(reader: event_data::Reader) -> capnp::Result<Self> {
    match reader.which()? {
      event_data::StoreUnlocked(args) => Ok(EventData::StoreUnlocked {
        store_name: args.get_store_name()?.to_string(),
        identity: Identity::from_reader(args.get_identity()?)?,
      }),
      event_data::StoreLocked(args) => Ok(EventData::StoreLocked {
        store_name: args.get_store_name()?.to_string(),
      }),
      event_data::SecretOpened(args) => Ok(EventData::SecretOpened {
        store_name: args.get_store_name()?.to_string(),
        identity: Identity::from_reader(args.get_identity()?)?,
        secret_id: args.get_secret_id()?.to_string(),
      }),
      event_data::SecretVersionAdded(args) => Ok(EventData::SecretVersionAdded {
        store_name: args.get_store_name()?.to_string(),
        identity: Identity::from_reader(args.get_identity()?)?,
        secret_id: args.get_secret_id()?.to_string(),
      }),
      event_data::IdentityAdded(args) => Ok(EventData::IdentityAdded {
        store_name: args.get_store_name()?.to_string(),
        identity: Identity::from_reader(args.get_identity()?)?,
      }),
      event_data::ClipboardProviding(args) => Ok(EventData::ClipboardProviding {
        store_name: args.get_store_name()?.to_string(),
        block_id: args.get_block_id()?.to_string(),
        property: args.get_property()?.to_string(),
      }),
      event_data::ClipboardDone(_) => Ok(EventData::ClipboardDone),
    }
  }

  fn to_builder(&self, mut builder: event_data::Builder) -> capnp::Result<()> {
    match self {
      EventData::StoreUnlocked { store_name, identity } => {
        let mut store_unlocked = builder.init_store_unlocked();
        store_unlocked.set_store_name(store_name);
        identity.to_builder(store_unlocked.init_identity())?;
      }
      EventData::StoreLocked { store_name } => {
        let mut store_locked = builder.init_store_locked();
        store_locked.set_store_name(store_name);
      }
      EventData::SecretOpened {
        store_name,
        identity,
        secret_id,
      } => {
        let mut secret_opened = builder.init_secret_opened();
        secret_opened.set_store_name(store_name);
        secret_opened.set_secret_id(secret_id);
        identity.to_builder(secret_opened.init_identity())?;
      }
      EventData::SecretVersionAdded {
        store_name,
        identity,
        secret_id,
      } => {
        let mut secret_version_added = builder.init_secret_version_added();
        secret_version_added.set_store_name(store_name);
        secret_version_added.set_secret_id(secret_id);
        identity.to_builder(secret_version_added.init_identity())?;
      }
      EventData::IdentityAdded { store_name, identity } => {
        let mut identity_added = builder.init_identity_added();
        identity_added.set_store_name(store_name);
        identity.to_builder(identity_added.init_identity())?;
      }
      EventData::ClipboardProviding {
        store_name,
        block_id,
        property,
      } => {
        let mut clipboard_providing = builder.init_clipboard_providing();
        clipboard_providing.set_store_name(store_name);
        clipboard_providing.set_block_id(block_id);
        clipboard_providing.set_property(property);
      }
      EventData::ClipboardDone => {
        builder.set_clipboard_done(());
      }
    }
    Ok(())
  }
}

pub trait EventHub: Send + Sync {
  fn send(&self, event: EventData);
}

#[derive(Clone, Debug, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct Event {
  pub id: u64,
  pub data: EventData,
}

impl CapnpSerializing for Event {
  type Owned = event::Owned;

  fn from_reader(reader: event::Reader) -> capnp::Result<Self> {
    Ok(Event {
      id: reader.get_id(),
      data: EventData::from_reader(reader.get_data()?)?,
    })
  }

  fn to_builder(&self, mut builder: event::Builder) -> capnp::Result<()> {
    builder.set_id(self.id);
    self.data.to_builder(builder.init_data())?;
    Ok(())
  }
}
