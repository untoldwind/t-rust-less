#[macro_use]
pub mod macros;

pub mod api;
pub mod block_store;
pub mod clipboard;
pub mod memguard;
pub mod secrets_store;

#[allow(dead_code)]
#[allow(clippy::wrong_self_convention)]
mod secrets_store_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/secrets_store/secrets_store_capnp.rs"));
}
