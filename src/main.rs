extern crate byteorder;
extern crate clap;
extern crate chrono;
extern crate data_encoding;
#[macro_use]
extern crate error_chain;
extern crate openssl;

mod cli;
mod ex_crypto;
mod store;

fn main() {
    cli::cli_run()
}
