use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

mod initialize;
mod sync;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Remote {
  #[cfg(feature = "dropbox")]
  Dropbox,
  #[cfg(feature = "pcloud")]
  Pcloud,
}

#[cfg(feature = "pcloud")]
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum PcloudRegion {
  Eu,
  Us,
}

#[derive(Debug, Parser)]
pub struct Args {
  #[clap(short, long, help = "Enable debug logs")]
  pub debug: bool,

  #[clap(short, long, help = "select supported remote type")]
  pub remote: Remote,

  #[cfg(feature = "pcloud")]
  #[clap(long, default_value = "eu", help = "pcloud region")]
  pub pcloud_region: PcloudRegion,

  #[cfg(feature = "dropbox")]
  #[clap(long, help = "dropbox auth token")]
  pub dropbox_token: Option<String>,

  #[cfg(feature = "pcloud")]
  #[clap(long, env = "PCLOUD_TOKEN", help = "pcloud auth token")]
  pub pcloud_token: Option<String>,

  #[clap(subcommand)]
  pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
  Initialize,
  Sync,
}

impl Command {
  pub async fn run(&self, args: &Args) -> Result<()> {
    match self {
      Command::Initialize => initialize::initialize(args).await,
      Command::Sync => sync::sync(args).await,
    }
  }
}
