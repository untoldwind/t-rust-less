use std::env;
use std::fs;
use std::process;

fn main() {
  capnpc::CompilerCommand::new()
    .file("src/secrets_store/secrets_store.capnp")
    .edition(capnpc::RustEdition::Rust2018)
    .run()
    .expect("compiling schema");
}
