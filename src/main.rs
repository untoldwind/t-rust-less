extern crate clap;
extern crate data_encoding;
#[macro_use]
extern crate error_chain;
extern crate openssl;

mod cli;
mod ex_crypto;

fn main() {
    cli::cli_run()
}
