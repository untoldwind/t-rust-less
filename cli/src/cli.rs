use clap::{App, AppSettings, Arg, SubCommand};

pub fn app() -> App<'static, 'static> {
  App::new("t-rust-less")
    .version(clap::crate_version!())
    .about("Manages passwords")
    .setting(AppSettings::ArgRequiredElseHelp)
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
    .subcommand(SubCommand::with_name("init").about("Initialize configuration and store (if necessary)"))
    .subcommand(
      SubCommand::with_name("identities")
        .alias("ids")
        .about("Control identities of a store")
        .subcommand(SubCommand::with_name("list").alias("ls").about("List identities"))
        .subcommand(SubCommand::with_name("add").about("Add a new identity")),
    )
    .subcommand(SubCommand::with_name("status").about("Show current status of the password store"))
    .subcommand(
      SubCommand::with_name("import")
        .about("Import secrets entries")
        .arg(
          Arg::with_name("v1")
            .long("v1")
            .help("Import V1 format (from original trustless)"),
        )
        .arg(Arg::with_name("file").help("File to import. If not set import will read from stdin")),
    )
    .subcommand(SubCommand::with_name("export").about("Export an entire store"))
    .subcommand(SubCommand::with_name("lock").about("Lock the store"))
    .subcommand(SubCommand::with_name("unlock").about("Unlock the store"))
    .subcommand(
      SubCommand::with_name("list")
        .alias("ls")
        .about("List secrets")
        .arg(
          Arg::with_name("name")
            .value_name("name-filter")
            .help("Fuzzy name filter"),
        )
        .arg(
          Arg::with_name("url")
            .long("url")
            .short("u")
            .value_name("url-filter")
            .number_of_values(1),
        )
        .arg(
          Arg::with_name("tag")
            .long("tag")
            .short("t")
            .value_name("tag-filter")
            .number_of_values(1),
        )
        .arg(Arg::with_name("deleted").long("deleted").help("List deleted items")),
    )
    .subcommand(
      SubCommand::with_name("generate")
        .about("Generate password")
        .arg(Arg::with_name("words").long("words"))
        .arg(
          Arg::with_name("length")
            .long("length")
            .value_name("length")
            .number_of_values(1),
        )
        .arg(
          Arg::with_name("delim")
            .long("delim")
            .value_name("delim")
            .number_of_values(1),
        )
        .arg(Arg::with_name("exclude-uppers").long("exclude-uppers"))
        .arg(Arg::with_name("exclude-numbers").long("exclude-numbers"))
        .arg(Arg::with_name("exclude-symbols").long("exclude-symbols"))
        .arg(Arg::with_name("require-upper").long("require-upper"))
        .arg(Arg::with_name("require-number").long("require-number"))
        .arg(Arg::with_name("require-symbol").long("require-symbol"))
        .arg(Arg::with_name("include-ambiguous").long("include-ambiguous"))
        .arg(Arg::with_name("include-similar").long("include-similar"))
        .arg(
          Arg::with_name("count")
            .value_name("count")
            .number_of_values(1)
            .default_value("5"),
        ),
    )
}
