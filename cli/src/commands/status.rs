use crate::error::ExtResult;
use atty::Stream;
use crossterm_style::{style, Color};
use std::sync::Arc;
use t_rust_less_lib::service::TrustlessService;

pub fn status(service: Arc<dyn TrustlessService>, store_name: String) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));
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
