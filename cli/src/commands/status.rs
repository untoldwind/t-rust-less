use crate::config::Config;
use crate::error::ExtResult;
use atty::Stream;
use colored::*;
use t_rust_less_lib::secrets_store::open_secrets_store;

pub fn status(config: Config) {
  let secrets_store =
    open_secrets_store(&config.store_url, &config.client_id, config.autolock_timeout).ok_or_exit("Open store");
  let status = secrets_store.status().ok_or_exit("Get status");

  if atty::is(Stream::Stdout) {
    println!();
    println!("Client version: {}", env!("CARGO_PKG_VERSION").cyan(),);
    println!("Store version : {}", status.version.cyan());
    println!(
      "Status        : {}",
      if status.locked {
        "Locked".green()
      } else {
        "Unlocked".red()
      }
    )
  } else {
    println!("Client version: {}", env!("CARGO_PKG_VERSION"));
    println!("Store version : {}", status.version);
  }
}
