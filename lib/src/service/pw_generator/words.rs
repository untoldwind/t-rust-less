use super::wordlist::WORDLIST;
use crate::api::PasswordGeneratorWordsParam;
use itertools::Itertools;
use rand::seq::SliceRandom;
use rand::thread_rng;

pub fn generate_words(params: &PasswordGeneratorWordsParam) -> String {
  let mut rng = thread_rng();

  WORDLIST
    .choose_multiple(&mut rng, params.num_words as usize)
    .join(&params.delim.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;
  use spectral::prelude::*;

  #[test]
  fn test_generate_words() {
    let pw1 = generate_words(&PasswordGeneratorWordsParam {
      num_words: 3,
      delim: '.',
    });

    assert_that(&pw1.len()).is_greater_than(5);
    assert_that(&pw1.split(".").count()).is_equal_to(3);

    let pw2 = generate_words(&PasswordGeneratorWordsParam {
      num_words: 5,
      delim: '-',
    });

    assert_that(&pw2.len()).is_greater_than(9);
    assert_that(&pw2.split("-").count()).is_equal_to(5);
  }
}
