use crate::clipboard::SelectionProvider;

pub mod api;
pub mod secrets_store;
#[allow(dead_code)]
mod secrets_store_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/secrets_store/secrets_store_capnp.rs"));
}
pub mod clipboard;
pub mod memguard;
pub mod store;
