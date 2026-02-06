mod cli;

mod autolock;
mod processor;
mod sync_trigger;

#[cfg(unix)]
mod unix;
use clap::Parser;
#[cfg(unix)]
use unix::run_server;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::run_server;

use std::{error::Error, sync::Arc};
use t_rust_less_lib::service::{config::LocalConfigProvider, local::LocalTrustlessService, TrustlessService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  let args = cli::Args::parse();

  #[cfg(not(unix))]
  init_console_logger(args.debug);
  #[cfg(unix)]
  if args.journal {
    init_systemd_logger(args.debug);
  } else {
    init_console_logger(args.debug);
  }

  let service = Arc::new(LocalTrustlessService::new(LocalConfigProvider::default())?);
  if service.needs_synchronization() {
    sync_trigger::start_sync_loop(service.clone());
  }
  autolock::start_autolock_loop(service.clone());

  run_server(service).await
}

fn init_console_logger(debug: bool) {
  let mut log_builder = env_logger::Builder::from_default_env();

  if debug {
    log_builder.filter(None, log::LevelFilter::Debug);
  } else {
    log_builder.filter(None, log::LevelFilter::Info);
  }
  log_builder.target(env_logger::Target::Stderr);
  log_builder.init();
}

#[cfg(unix)]
fn init_systemd_logger(debug: bool) {
  systemd_journal_logger::init().unwrap();

  if debug {
    log::set_max_level(log::LevelFilter::Debug);
  } else {
    log::set_max_level(log::LevelFilter::Info);
  }
}
