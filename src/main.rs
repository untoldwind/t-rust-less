mod api;
mod cli;
mod secret_store;
mod secret_store_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/secret_store/secret_store_capnp.rs"));
}
mod store;
mod memguard;

fn main() {
  cli::cli_run()
}
