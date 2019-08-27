fn main() {
  capnpc::CompilerCommand::new()
    .file("src/api/api.capnp")
    .run()
    .expect("compiling schema");

  capnpc::CompilerCommand::new()
    .file("src/secrets_store/secrets_store.capnp")
    .run()
    .expect("compiling schema");
}
