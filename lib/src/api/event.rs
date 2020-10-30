use crate::api::Identity;
use crate::api_capnp::{event, EventType};
use serde_derive::{Deserialize, Serialize};
use zeroize::Zeroize;

#[derive(Clone, Debug, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub enum Event {
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

impl Event {
  pub fn from_reader(reader: event::Reader) -> capnp::Result<Self> {
    match reader.get_type()? {
      EventType::StoreUnlocked => Ok(Event::StoreUnlocked {
        store_name: reader.get_store_name()?.to_string(),
        identity: Identity::from_reader(reader.get_identity()?)?,
      }),
      EventType::StoreLocked => Ok(Event::StoreLocked {
        store_name: reader.get_store_name()?.to_string(),
      }),
      EventType::SecretOpened => Ok(Event::SecretOpened {
        store_name: reader.get_store_name()?.to_string(),
        identity: Identity::from_reader(reader.get_identity()?)?,
        secret_id: reader.get_secret_id()?.to_string(),
      }),
      EventType::SecretVersionAdded => Ok(Event::SecretVersionAdded {
        store_name: reader.get_store_name()?.to_string(),
        identity: Identity::from_reader(reader.get_identity()?)?,
        secret_id: reader.get_secret_id()?.to_string(),
      }),
      EventType::IdentityAdded => Ok(Event::IdentityAdded {
        store_name: reader.get_store_name()?.to_string(),
        identity: Identity::from_reader(reader.get_identity()?)?,
      }),
      EventType::ClipboardProviding => Ok(Event::ClipboardProviding {
        store_name: reader.get_store_name()?.to_string(),
        block_id: reader.get_block_id()?.to_string(),
        property: reader.get_property()?.to_string(),
      }),
      EventType::ClipboardDone => Ok(Event::ClipboardDone),
    }
  }

  pub fn to_builder(&self, mut builder: event::Builder) -> capnp::Result<()> {
    match &self {
      Event::StoreUnlocked { store_name, identity } => {
        builder.set_type(EventType::StoreUnlocked);
        builder.set_store_name(store_name);
        identity.to_builder(builder.init_identity());
      }
      Event::StoreLocked { store_name } => {
        builder.set_type(EventType::StoreLocked);
        builder.set_store_name(store_name);
      }
      Event::SecretOpened {
        store_name,
        identity,
        secret_id,
      } => {
        builder.set_type(EventType::SecretOpened);
        builder.set_store_name(store_name);
        builder.set_secret_id(secret_id);
        identity.to_builder(builder.init_identity());
      }
      Event::SecretVersionAdded {
        store_name,
        identity,
        secret_id,
      } => {
        builder.set_type(EventType::SecretVersionAdded);
        builder.set_store_name(store_name);
        builder.set_secret_id(secret_id);
        identity.to_builder(builder.init_identity());
      }
      Event::IdentityAdded { store_name, identity } => {
        builder.set_type(EventType::IdentityAdded);
        builder.set_store_name(store_name);
        identity.to_builder(builder.init_identity());
      }
      Event::ClipboardProviding {
        store_name,
        block_id,
        property,
      } => {
        builder.set_type(EventType::ClipboardProviding);
        builder.set_store_name(store_name);
        builder.set_block_id(block_id);
        builder.set_property(property);
      }
      Event::ClipboardDone => {
        builder.set_type(EventType::ClipboardDone);
      }
    }
    Ok(())
  }
}

pub trait EventHandler {
  fn handle(&self, event: Event);
}

pub trait EventHub {
  fn send(&self, event: Event);
}

pub trait EventSubscription {}
