use clap::{App, Arg, SubCommand};

pub fn app() -> App<'static, 'static> {
  App::new("t-rust-less").version("0.1").about("Manages passwords")
}

pub fn cli_run() {
  let matches = app().get_matches();
}
