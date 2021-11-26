mod cli;

mod autolock;
mod error;
mod processor;
mod sync_trigger;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::run_server;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::run_server;

use std::{error::Error, sync::Arc};
use t_rust_less_lib::service::{local::LocalTrustlessService, TrustlessService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let matches = cli::app().get_matches();

  let mut log_builder = env_logger::Builder::from_default_env();

  if matches.is_present("debug") {
    log_builder.filter(None, log::LevelFilter::Debug);
  } else {
    log_builder.filter(None, log::LevelFilter::Info);
  }
  log_builder.target(env_logger::Target::Stderr);
  log_builder.init();

  let service = Arc::new(LocalTrustlessService::new()?);
  if service.needs_synchronization() {
    sync_trigger::start_sync_loop(service.clone());
  }
  autolock::start_autolock_loop(service.clone());

  run_server(service).await
}
