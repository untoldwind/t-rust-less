use crate::config::Config;
use crate::error::ExtResult;
use atty::Stream;
use crossterm_style::{style, Color};

pub fn status(config: Config) {
  let secrets_store = config.open_secrets_store();
  let status = secrets_store.status().ok_or_exit("Get status");

  if atty::is(Stream::Stdout) {
    println!();
    println!("Client version: {}", style(env!("CARGO_PKG_VERSION")).with(Color::Cyan));
    println!("Store version : {}", style(status.version).with(Color::Cyan));
    println!(
      "Status        : {}",
      if status.locked {
        style("Locked").with(Color::Green)
      } else {
        style("Unlocked").with(Color::Red)
      }
    )
  } else {
    println!("Client version: {}", env!("CARGO_PKG_VERSION"));
    println!("Store version : {}", status.version);
  }
}
