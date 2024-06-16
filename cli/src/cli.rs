use clap::Parser;

use crate::commands::MainCommand;

#[derive(Debug, Parser)]
#[clap(name = "t-rust-less", about = "Manages passwords", version = clap::crate_version!())]
pub struct Args {
  #[clap(short, long, help = "Enable debug logs")]
  pub debug: bool,

  #[clap(short, long, help = "Select store to use")]
  pub store: Option<String>,

  #[clap(subcommand)]
  pub sub_command: MainCommand,
}
