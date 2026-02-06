use clap::Parser;

#[derive(Parser, Debug)]
#[clap(name = "t-rust-less-daemon", about = "Manages passwords", version = clap::crate_version!())]
pub struct Args {
  #[clap(short, long, help = "Enable debug logs")]
  pub debug: bool,
  #[cfg(unix)]
  #[clap(long, help = "Log to systemd journal")]
  pub journal: bool,
}
