mod add_identity;
mod completions;
mod export;
mod generate;
mod import;
mod init;
mod list_identities;
mod list_secrets;
mod lock;
mod status;
pub mod tui;
mod unlock;

use anyhow::Result;
use std::process;
use std::sync::Arc;

pub use self::unlock::*;

use clap::Args;
use crossterm_style::{style, Color};

use atty::Stream;
use clap::Subcommand;
use log::error;
use rand::{distributions, thread_rng, Rng};
use t_rust_less_lib::service::config_file;
use t_rust_less_lib::service::TrustlessService;

fn generate_id(length: usize) -> String {
  let rng = thread_rng();

  rng
    .sample_iter(distributions::Alphanumeric)
    .map(char::from)
    .take(length)
    .collect::<String>()
}

#[derive(Debug, Subcommand)]
pub enum IdentitiesSubCommand {
  #[clap(about = "Add a new identity")]
  Add(add_identity::AddIdentitiesCommand),
  #[clap(about = "List identities", alias = "ls")]
  List(list_identities::ListIdentitiesCommand),
}

#[derive(Debug, Args)]
pub struct IdentitiesCommand {
  #[clap(subcommand)]
  subcommand: IdentitiesSubCommand,
}

impl IdentitiesCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) -> Result<()> {
    match self.subcommand {
      IdentitiesSubCommand::Add(cmd) => cmd.run(service, store_name),
      IdentitiesSubCommand::List(cmd) => cmd.run(service, store_name),
    }
  }
}

#[derive(Debug, Subcommand)]
pub enum MainCommand {
  #[clap(about = "Initialize configuration and store (if necessary)")]
  Init(init::InitCommand),
  #[clap(about = "Lock the store")]
  Lock(lock::LockCommand),
  #[clap(about = "Unlock the store")]
  Unlock(unlock::UnlockCommand),
  #[clap(about = "Import secrets entries")]
  Import(import::ImportCommand),
  #[clap(about = "Export secrets entries")]
  Export(export::ExportCommand),
  #[clap(about = "Show current status of the password store")]
  Status(status::StatusCommand),
  #[clap(about = "List secrets", alias = "ls")]
  List(list_secrets::ListSecretsCommand),
  #[clap(about = "Generate password")]
  Generate(generate::GenerateCommand),
  #[clap(about = "Control identities of a store", alias = "ids")]
  Identities(IdentitiesCommand),
  #[clap(about = "Generate shell completions")]
  Completions(completions::CompletionCommand),
}

impl MainCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, maybe_store_name: Option<String>) -> Result<()> {
    if let MainCommand::Init(cmd) = self {
      return cmd.run(service, maybe_store_name);
    }

    let store_name = match maybe_store_name {
      Some(store_name) => store_name,
      _ => {
        uninitialized();
        unreachable!()
      }
    };

    match self {
      MainCommand::Lock(cmd) => cmd.run(service, store_name),
      MainCommand::Unlock(cmd) => cmd.run(service, store_name),
      MainCommand::Import(cmd) => cmd.run(service, store_name),
      MainCommand::Export(cmd) => cmd.run(service, store_name),
      MainCommand::Status(cmd) => cmd.run(service, store_name),
      MainCommand::List(cmd) => cmd.run(service, store_name),
      MainCommand::Generate(cmd) => cmd.run(service),
      MainCommand::Identities(cmd) => cmd.run(service, store_name),
      MainCommand::Completions(cmd) => cmd.run(),
      _ => Ok(()),
    }
  }
}

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
