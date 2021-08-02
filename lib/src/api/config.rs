use crate::api_capnp::store_config;
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

use super::{read_option, CapnpSerializing};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Zeroize)]
#[zeroize(drop)]
pub struct StoreConfig {
  pub name: String,
  pub store_url: String,
  pub client_id: String,
  pub autolock_timeout_secs: u64,
  pub default_identity_id: Option<String>,
}

impl CapnpSerializing for StoreConfig {
  type Owned = store_config::Owned;

  fn from_reader(reader: store_config::Reader) -> capnp::Result<StoreConfig> {
    Ok(StoreConfig {
      name: reader.get_name()?.to_string(),
      store_url: reader.get_store_url()?.to_string(),
      client_id: reader.get_client_id()?.to_string(),
      autolock_timeout_secs: reader.get_autolock_timeout_secs(),
      default_identity_id: read_option(reader.get_default_identity_id()?)?.map(ToString::to_string),
    })
  }

  fn to_builder(&self, mut builder: store_config::Builder) -> capnp::Result<()> {
    builder.set_name(&self.name);
    builder.set_store_url(&self.store_url);
    builder.set_client_id(&self.client_id);
    builder.set_autolock_timeout_secs(self.autolock_timeout_secs);
    match &self.default_identity_id {
      Some(default_identity_id) => builder
        .reborrow()
        .init_default_identity_id()
        .set_some(capnp::text::new_reader(default_identity_id.as_bytes())?)?,
      None => builder.reborrow().init_default_identity_id().set_none(()),
    }

    Ok(())
  }
}
