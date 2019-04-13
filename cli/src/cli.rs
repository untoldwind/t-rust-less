use clap::{App, Arg, SubCommand};

pub fn app() -> App<'static, 'static> {
  App::new("t-rust-less")
    .version("0.1")
    .about("Manages passwords")
    .arg(
      Arg::with_name("debug")
        .short("D")
        .long("debug")
        .help("Enable debug logs"),
    )
    .subcommand(SubCommand::with_name("init").about("Initialize configuration and store (if necessary"))
    .subcommand(SubCommand::with_name("status").about("Show current status of the password store"))
}
