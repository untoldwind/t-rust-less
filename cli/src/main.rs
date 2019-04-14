use crate::config::{config_file, read_config};
use atty::Stream;
use log::error;
use termion::{color, style};

mod cli;
mod commands;
mod config;
mod error;

fn uninitialized() {
  if atty::is(Stream::Stdout) {
    println!();
    println!(
      "{}{}Missing configuration{}{}",
      color::Fg(color::Red),
      style::Bold,
      color::Fg(color::Reset),
      style::Reset
    );
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

  if let Some(_) = matches.subcommand_matches("init") {
    commands::init(maybe_config);
  } else {
    match maybe_config {
      Some(config) => {
        if let Some(_) = matches.subcommand_matches("status") {
          commands::status();
        }
      }
      None => uninitialized(),
    }
  }
}