use clap::{App, Arg};

pub fn app() -> App<'static, 'static> {
  App::new("t-rust-less").version("0.1").about("Manages passwords").arg(
    Arg::with_name("debug")
      .short("D")
      .long("debug")
      .help("Enable debug logs"),
  )
}
