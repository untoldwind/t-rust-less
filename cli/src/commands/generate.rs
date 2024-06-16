use crate::error::ExtResult;
use clap::Args;
use std::sync::Arc;
use t_rust_less_lib::{
  api::{PasswordGeneratorCharsParam, PasswordGeneratorParam, PasswordGeneratorWordsParam},
  service::TrustlessService,
};

#[derive(Debug, Args)]
pub struct GenerateCommand {
  #[clap(long)]
  exclude_uppers: bool,
  #[clap(long)]
  exclude_numbers: bool,
  #[clap(long)]
  exclude_symbols: bool,
  #[clap(long)]
  require_upper: bool,
  #[clap(long)]
  require_number: bool,
  #[clap(long)]
  require_symbol: bool,
  #[clap(long)]
  include_ambiguous: bool,
  #[clap(long)]
  include_similar: bool,
  #[clap(long)]
  words: bool,
  #[clap(long, default_value = ".")]
  delim: String,
  #[clap(long)]
  length: Option<u8>,
  #[clap(long, default_value = "5")]
  count: usize,
}

impl GenerateCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>) {
    let param: PasswordGeneratorParam = if self.words {
      PasswordGeneratorParam::Words(PasswordGeneratorWordsParam {
        num_words: self.length.unwrap_or(4),
        delim: self.delim.chars().next().unwrap_or('.'),
      })
    } else {
      PasswordGeneratorParam::Chars(PasswordGeneratorCharsParam {
        num_chars: self.length.unwrap_or(16),
        include_uppers: !self.exclude_uppers,
        include_numbers: !self.exclude_numbers,
        include_symbols: !self.exclude_symbols,
        require_upper: self.require_upper,
        require_number: self.require_number,
        require_symbol: self.require_symbol,
        exclude_ambiguous: !self.include_ambiguous,
        exclude_similar: !self.include_similar,
      })
    };

    for _ in 0..self.count {
      println!(
        "{}",
        service.generate_password(param.clone()).ok_or_exit("Generate password")
      );
    }
  }
}
