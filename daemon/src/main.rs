mod cli;

#[macro_use]
pub mod macros;
mod error;
mod secrets_store_impl;
mod service_impl;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::run_server;

use crate::error::ExtResult;
use log::info;
use std::sync::Arc;
use t_rust_less_lib::api_capnp::service;
use t_rust_less_lib::service::local::LocalTrustlessService;

fn main() {
  let matches = cli::app().get_matches();

  let mut log_builder = env_logger::Builder::from_default_env();

  if matches.is_present("debug") {
    log_builder.filter(None, log::LevelFilter::Debug);
  } else {
    log_builder.filter(None, log::LevelFilter::Info);
  }
  log_builder.target(env_logger::Target::Stderr);
  log_builder.init();

  let service = Arc::new(LocalTrustlessService::new().ok_or_exit("Open local store"));
  let service_server =
    service::ToClient::new(service_impl::ServiceImpl::new(service.clone())).into_client::<capnp_rpc::Server>();

  run_server(
    move || {
      info!("New client connection");
      service_server.clone().client
    },
    move || service.check_autolock(),
  );
}
