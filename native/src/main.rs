use log::error;
use std::io::{stdin, stdout};
use std::process;
use t_rust_less_lib::service::create_service;

mod messages;
mod output;
mod processor;

fn main() {
  env_logger::Builder::from_default_env()
    .filter(None, log::LevelFilter::Debug)
    .target(env_logger::Target::Stderr)
    .init();

  let service = match create_service() {
    Ok(service) => service,
    Err(error) => {
      error!("Failed creating service: {}", error);
      process::exit(1);
    }
  };

  let mut processor = match processor::Processor::new(service, stdin(), stdout()) {
    Ok(processor) => processor,
    Err(error) => {
      error!("Failed creating processor: {}", error);
      process::exit(1);
    }
  };

  if let Err(error) = processor.process() {
    error!("Error: {}", error);
    process::exit(1);
  }
}
