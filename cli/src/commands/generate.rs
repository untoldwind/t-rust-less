use crate::error::ExtResult;
use clap::ArgMatches;
use std::sync::Arc;
use t_rust_less_lib::{
  api::{PasswordGeneratorCharsParam, PasswordGeneratorParam, PasswordGeneratorWordsParam},
  service::TrustlessService,
};

pub fn password_generate_param_from_args(args: &ArgMatches) -> PasswordGeneratorParam {
  if args.is_present("words") {
    PasswordGeneratorParam::Words(PasswordGeneratorWordsParam {
      num_words: args.value_of("length").and_then(|v| v.parse::<u8>().ok()).unwrap_or(4),
      delim: args.value_of("delim").and_then(|v| v.chars().next()).unwrap_or('.'),
    })
  } else {
    PasswordGeneratorParam::Chars(PasswordGeneratorCharsParam {
      num_chars: args.value_of("length").and_then(|v| v.parse::<u8>().ok()).unwrap_or(16),
      include_uppers: !args.is_present("exclude-uppers"),
      include_numbers: !args.is_present("exclude-numbers"),
      include_symbols: !args.is_present("exclude-symbols"),
      require_upper: args.is_present("require-upper"),
      require_number: args.is_present("require-number"),
      require_symbol: args.is_present("require-symbol"),
      exclude_ambiguous: !args.is_present("include-ambiguous"),
      exclude_similar: !args.is_present("include-similar"),
    })
  }
}

pub fn generate(service: Arc<dyn TrustlessService>, param: PasswordGeneratorParam, count: usize) {
  for _ in 0..count {
    println!(
      "{}",
      service.generate_password(param.clone()).ok_or_exit("Generate password")
    );
  }
}
