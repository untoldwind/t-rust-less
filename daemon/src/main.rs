mod cli;

mod error;
mod processor;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::run_server;
#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::run_server;

use std::{error::Error, sync::Arc, time::Duration};
use t_rust_less_lib::service::local::LocalTrustlessService;
use t_rust_less_lib::service::TrustlessService;
use tokio::time::interval;

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

  let mut interval = interval(Duration::from_secs(1));
  let service_cloned = service.clone();
  tokio::spawn(async move {
    loop {
      interval.tick().await;
      service_cloned.check_autolock();
    }
  });

  run_server(service).await
}
