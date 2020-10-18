mod cli;

#[macro_use]
pub mod macros;
mod clipboard_control_impl;
mod error;
mod event_handler_impl;
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
use t_rust_less_lib::service::local::{LocalTrustlessService, TrustlessService};

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

  run_server(
    {
      let cloned = service.clone();
      move || {
        info!("New client connection");
        let service_server: service::Client = capnp_rpc::new_client(service_impl::ServiceImpl::new(cloned.clone()));

        service_server.client
      }
    },
    move || service.check_autolock(),
  );
}
