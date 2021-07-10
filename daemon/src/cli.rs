use clap::{App, Arg};

pub fn app() -> App<'static, 'static> {
  App::new("t-rust-less")
    .version(clap::crate_version!())
    .about("Manages passwords")
    .arg(
      Arg::with_name("debug")
        .short("D")
        .long("debug")
        .help("Enable debug logs"),
    )
}
