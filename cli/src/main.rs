use crate::error::ExtResult;
use anyhow::Result;
use clap::Parser;
use t_rust_less_lib::service::create_service;

mod cli;
mod commands;
mod config;
mod error;
pub mod model;
pub mod view;

fn main() -> Result<()> {
  let args = cli::Args::parse();

  let mut log_builder = env_logger::Builder::from_default_env();

  if args.debug {
    log_builder.filter(None, log::LevelFilter::Debug);
  } else {
    log_builder.filter(None, log::LevelFilter::Error);
  }
  log_builder.target(env_logger::Target::Stderr);
  log_builder.init();

  let service = create_service().ok_or_exit("Failed creating service");

  let maybe_store_name = args
    .store
    .or_else(|| service.get_default_store().ok_or_exit("Get default store"));

  args.sub_command.run(service, maybe_store_name)
}
