mod api;
mod cli;
mod secrets;
mod secrets_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/secrets/secrets_capnp.rs"));
}
mod store;

fn main() {
  cli::cli_run()
}
