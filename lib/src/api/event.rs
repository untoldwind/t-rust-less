use crate::api::Identity;
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

pub trait EventHub: Send + Sync {
  fn send(&self, event: EventData);
}

#[derive(Clone, Debug, Serialize, Deserialize, Zeroize)]
#[zeroize(drop)]
pub struct Event {
  pub id: u64,
  pub data: EventData,
}
