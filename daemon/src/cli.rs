use clap::{App, AppSettings, Arg, SubCommand};

pub fn app() -> App<'static, 'static> {
  App::new("t-rust-less")
    .version("0.1")
    .about("Manages passwords")
    .setting(AppSettings::ArgRequiredElseHelp)
    .arg(
      Arg::with_name("debug")
        .short("D")
        .long("debug")
        .help("Enable debug logs"),
    )
}
