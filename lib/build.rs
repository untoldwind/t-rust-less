fn main() {
  capnpc::CompilerCommand::new()
    .file("src/api/api.capnp")
    .edition(capnpc::RustEdition::Rust2018)
    .run()
    .expect("compiling schema");

  capnpc::CompilerCommand::new()
    .file("src/secrets_store/secrets_store.capnp")
    .edition(capnpc::RustEdition::Rust2018)
    .run()
    .expect("compiling schema");
}
