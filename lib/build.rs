fn main() {
  capnpc::CompilerCommand::new()
    .file("src/secrets_store/secrets_store.capnp")
    .edition(capnpc::RustEdition::Rust2018)
    .run()
    .expect("compiling schema");

  capnpc::CompilerCommand::new()
    .file("src/service/service.capnp")
    .edition(capnpc::RustEdition::Rust2018)
    .run()
    .expect("compiling schema");
}
