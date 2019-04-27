mod cli;

mod error;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::run_server;

use capnp::capability::Promise;
use log::info;
use t_rust_less_lib::service_capnp::service;
use t_rust_less_lib::service::local::LocalTrustlessService;
use crate::error::ExtResult;
use t_rust_less_lib::service::TrustlessService;

struct ServiceImpl {
  service: LocalTrustlessService,
}

impl ServiceImpl {
  fn new() -> Self {
    ServiceImpl {
      service: LocalTrustlessService::new().ok_or_exit("Open local store"),
    }
  }
}

impl service::Server for ServiceImpl {
  fn list_stores(&mut self, _: service::ListStoresParams, mut results: service::ListStoresResults) -> Promise<(), capnp::Error> {
    let mut store_names = results.get().init_store_names(1);
    store_names.set(0, "bla");

    Promise::ok(())
  }
}

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

  let service_server = service::ToClient::new(ServiceImpl::new()).into_client::<capnp_rpc::Server>();

  run_server(move || {
    info!("New client connection");
    service_server.clone().client
  });
}
