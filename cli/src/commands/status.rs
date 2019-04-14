use crate::config::Config;
use crate::error::ExtResult;
use atty::Stream;
use t_rust_less_lib::secrets_store::open_secrets_store;
use termion::color;

pub fn status(config: Config) {
  let secrets_store = open_secrets_store(&config.store_url).ok_or_exit("Open store");
  let status = secrets_store.status().ok_or_exit("Get status");

  if atty::is(Stream::Stdout) {
    println!();
    println!(
      "{}Client version: {}{}",
      color::Fg(color::Reset),
      color::Fg(color::Cyan),
      env!("CARGO_PKG_VERSION")
    );
    println!(
      "{}Store version : {}{}",
      color::Fg(color::Reset),
      color::Fg(color::Cyan),
      status.version
    );
    if status.locked {
      println!(
        "{}Status        : {}Locked",
        color::Fg(color::Reset),
        color::Fg(color::Green)
      )
    } else {
      println!(
        "{}Status        : {}Unlocked",
        color::Fg(color::Reset),
        color::Fg(color::Red)
      )
    }
  } else {
    println!("Client version: {}", env!("CARGO_PKG_VERSION"));
    println!("Store version : {}", status.version);
  }
}
