use crate::error::ExtResult;
use atty::Stream;
use crossterm_style::{style, Color};
use log::error;
use std::process;
use t_rust_less_lib::service::{config_file, create_service};

mod cli;
mod commands;
mod config;
mod error;
mod model;

fn uninitialized() {
  if atty::is(Stream::Stdout) {
    println!();
    println!("{}", style("No default store found").with(Color::Red));
    println!();
    println!(
      "t-rust-less was unable to find a default store in configuration at '{}'.",
      config_file().to_string_lossy()
    );
    println!("Probably t-rust-less has not been initialized yet. You may fix this problem with 't-rust-less init'");
    println!();
  } else {
    error!(
      "Missing default store in configuration: {}",
      config_file().to_string_lossy()
    );
  }
  process::exit(1)
}

fn main() {
  let matches = cli::app().get_matches();

  let mut log_builder = env_logger::Builder::from_default_env();

  if matches.is_present("debug") {
    log_builder.filter(None, log::LevelFilter::Debug);
  } else {
    log_builder.filter(None, log::LevelFilter::Error);
  }
  log_builder.target(env_logger::Target::Stderr);
  log_builder.init();

  let service = create_service().ok_or_exit("Failed opening servier");
  let maybe_store_name = matches
    .value_of("store")
    .map(str::to_string)
    .or_else(|| service.get_default_store().ok_or_exit("Get default store"));

  if matches.subcommand_matches("init").is_some() {
    commands::init(service, maybe_store_name);
    return;
  }
  let store_name = match maybe_store_name {
    Some(store_name) => store_name,
    _ => {
      uninitialized();
      unreachable!()
    }
  };

  if matches.subcommand_matches("status").is_some() {
    commands::status(service, store_name);
    return;
  }
  if let Some(sub_matches) = matches.subcommand_matches("identities") {
    if sub_matches.subcommand_matches("add").is_some() {
      commands::add_identity(service, store_name);
      return;
    }
    if sub_matches.subcommand_matches("list").is_some() {
      commands::list_identities(service, store_name);
      return;
    }
  }


}
