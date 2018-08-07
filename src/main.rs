extern crate byteorder;
extern crate chrono;
extern crate clap;
extern crate data_encoding;
#[macro_use]
extern crate error_chain;
extern crate openssl;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

mod cli;
mod ex_crypto;
mod store;

fn main() {
    cli::cli_run()
}
