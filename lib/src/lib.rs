#[macro_use]
pub mod macros;

pub mod api;
pub mod block_store;
pub mod clipboard;
pub mod memguard;
pub mod otp;
pub mod secrets_store;
pub mod service;

#[allow(dead_code)]
#[allow(
  clippy::wrong_self_convention,
  clippy::redundant_closure,
  clippy::redundant_field_names,
  clippy::match_single_binding
)]
mod secrets_store_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/secrets_store/secrets_store_capnp.rs"));
}

#[allow(dead_code)]
#[allow(
  clippy::wrong_self_convention,
  clippy::redundant_closure,
  clippy::redundant_field_names,
  clippy::match_single_binding
)]
pub mod api_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/api/api_capnp.rs"));
}
