use std::env;
use std::fs;
use std::process;

use clap::Shell;

#[path = "src/cli/mod.rs"]
mod cli;

fn main() {
  let outdir = match env::var_os("OUT_DIR") {
    Some(outdir) => outdir,
    None => {
      eprintln!("OUT_DIR environment variable not defined.");
      process::exit(1);
    }
  };
  fs::create_dir_all(&outdir).unwrap();

  let mut app = cli::app();
  app.gen_completions("t-rust-less", Shell::Bash, &outdir);
  app.gen_completions("t-rust-less", Shell::Fish, &outdir);
  app.gen_completions("t-rust-less", Shell::Zsh, &outdir);
  app.gen_completions("t-rust-less", Shell::PowerShell, &outdir);

  capnpc::CompilerCommand::new()
    .file("src/secret_store/secret_store.capnp")
    .edition(capnpc::RustEdition::Rust2018)
    .run()
    .expect("compiling schema");
}
