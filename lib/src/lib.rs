#[macro_use]
pub mod macros;

#[macro_use]
#[cfg(test)]
extern crate hex_literal;

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
pub mod secrets_store_capnp;

#[allow(dead_code)]
#[allow(
  clippy::wrong_self_convention,
  clippy::redundant_closure,
  clippy::redundant_field_names,
  clippy::match_single_binding
)]
pub mod api_capnp;
