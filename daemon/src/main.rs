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

use log::info;
use t_rust_less_lib::service_capnp::service;

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

  let service_server = service::ToClient::new(service_impl::ServiceImpl::new()).into_client::<capnp_rpc::Server>();

  run_server(move || {
    info!("New client connection");
    service_server.clone().client
  });
}
