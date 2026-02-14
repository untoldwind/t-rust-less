use anyhow::Result;
use clap::Parser;

mod cli;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
  let args = cli::Args::parse();

  let mut log_builder = env_logger::Builder::from_default_env();

  if args.debug {
    log_builder.filter(None, log::LevelFilter::Debug);
  } else {
    log_builder.filter(None, log::LevelFilter::Error);
  }
  log_builder.target(env_logger::Target::Stderr);
  log_builder.init();

  args.command.run(&args).await?;

  Ok(())
}
