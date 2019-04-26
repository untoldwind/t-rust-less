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
    .arg(
      Arg::with_name("store")
        .long("store")
        .value_name("name")
        .number_of_values(1)
        .help("Select store to use"),
    )
    .subcommand(SubCommand::with_name("init").about("Initialize configuration and store (if necessary"))
    .subcommand(
      SubCommand::with_name("identities")
        .alias("ids")
        .about("Control identities of a store")
        .subcommand(SubCommand::with_name("list").alias("ls").about("List identities"))
        .subcommand(SubCommand::with_name("add").about("Add a new identity")),
    )
    .subcommand(SubCommand::with_name("status").about("Show current status of the password store"))
    .subcommand(SubCommand::with_name("import").about("Import secrets entries"))
    .subcommand(SubCommand::with_name("export").about("Export an entire store"))
    .subcommand(SubCommand::with_name("lock").about("Lock the store"))
    .subcommand(SubCommand::with_name("unlock").about("Unlock the store"))
    .subcommand(SubCommand::with_name("daemon").about("Start background daemon"))
}
