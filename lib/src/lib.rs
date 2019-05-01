#[macro_use]
pub mod macros;

pub mod api;
pub mod block_store;
pub mod clipboard;
pub mod memguard;
pub mod secrets_store;
pub mod service;

#[allow(dead_code)]
#[allow(clippy::wrong_self_convention, clippy::redundant_closure)]
mod secrets_store_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/secrets_store/secrets_store_capnp.rs"));
}

#[allow(dead_code)]
#[allow(clippy::wrong_self_convention, clippy::redundant_closure)]
pub mod api_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/api/api_capnp.rs"));
}
