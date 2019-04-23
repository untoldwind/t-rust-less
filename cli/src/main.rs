use crate::config::{config_file, read_config};
use atty::Stream;
use crossterm_style::{style, Color};
use log::error;

mod cli;
mod commands;
mod config;
mod error;

fn uninitialized() {
  if atty::is(Stream::Stdout) {
    println!();
    println!("{}", style("Missing configuration").with(Color::Red));
    println!();
    println!(
      "t-rust-less was unable to find a configuration at '{}'.",
      config_file().to_string_lossy()
    );
    println!("Create this file manually or use 't-rust-less init'");
    println!();
  } else {
    error!("Missing configuration file: {}", config_file().to_string_lossy());
  }
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

  let maybe_config = read_config();

  if matches.subcommand_matches("init").is_some() {
    commands::init(maybe_config);
    return;
  }

  match maybe_config {
    Some(config) => {
      if matches.subcommand_matches("status").is_some() {
        commands::status(config);
      }
    }
    None => uninitialized(),
  }
}
