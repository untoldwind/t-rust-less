use clap::{App, Arg};

pub fn app() -> App<'static, 'static> {
  let app = App::new("t-rust-less")
    .version(clap::crate_version!())
    .about("Manages passwords")
    .arg(
      Arg::with_name("debug")
        .short("D")
        .long("debug")
        .help("Enable debug logs"),
    );

  #[cfg(unix)]
  let app = app.arg(Arg::with_name("journal").long("journal").help("Log to systemd journal"));

  app
}
