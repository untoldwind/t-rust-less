use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

mod initialize;

#[derive(Debug, Clone, ValueEnum)]
pub enum Remote {
  #[cfg(feature = "dropbox")]
  Dropbox,
  #[cfg(feature = "pcloud")]
  PCloud,
}

#[derive(Debug, Parser)]
pub struct Args {
  #[clap(short, long, help = "Enable debug logs")]
  pub debug: bool,

  #[clap(short, long, help = "select supported remote type")]
  pub remote: Remote,

  #[clap(subcommand)]
  pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
  Initialize,
}

impl Command {
  pub fn run(&self, remote: Remote) -> Result<()> {
    match self {
      Command::Initialize => initialize::initialize(remote),
    }
  }
}
