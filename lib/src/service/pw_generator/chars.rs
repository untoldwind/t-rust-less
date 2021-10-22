use crate::api::PasswordGeneratorCharsParam;
use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};

const LOWERS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const UPPERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERS: &[u8] = b"0123456789";
const SYMBOLS: &[u8] = b"!-+*#_$%&/()=?{}[]()/\\'\"`-,;:.<>";
const AMBIGOUS_CHARS: &[u8] = b"{}[]()/\\'\"`-,;:.<>";
const SIMILAR_CHARS: &[u8] = b"QO01lIB8S5G62ZUV";

pub fn generate_chars(params: &PasswordGeneratorCharsParam) -> String {
  let mut rng = thread_rng();
  let mut pool = Vec::with_capacity(params.num_chars as usize);

  if params.require_upper {
    pool.push(pick_char_from(&mut rng, UPPERS, &params));
  }
  if params.require_number {
    pool.push(pick_char_from(&mut rng, NUMBERS, &params));
  }
  if params.require_symbol {
    pool.push(pick_char_from(&mut rng, SYMBOLS, &params));
  }
  let candidates = create_base_set(&params);
  while pool.len() < params.num_chars as usize {
    pool.push(*candidates.choose(&mut rng).unwrap());
  }

  pool.shuffle(&mut rng);

  String::from_utf8(pool).unwrap()
}

fn create_base_set(params: &PasswordGeneratorCharsParam) -> Vec<u8> {
  let mut candidates = Vec::with_capacity(LOWERS.len() + UPPERS.len() + NUMBERS.len() + SYMBOLS.len());

  filter_set(&mut candidates, LOWERS, params);
  if params.include_uppers {
    filter_set(&mut candidates, UPPERS, params);
  }
  if params.include_numbers {
    filter_set(&mut candidates, NUMBERS, params);
  }
  if params.include_symbols {
    filter_set(&mut candidates, SYMBOLS, params);
  }

  candidates
}

fn filter_set(candidates: &mut Vec<u8>, set: &[u8], params: &PasswordGeneratorCharsParam) {
  for ch in set {
    if params.exclude_similar && SIMILAR_CHARS.contains(ch) {
      continue;
    }
    if params.exclude_ambiguous && AMBIGOUS_CHARS.contains(ch) {
      continue;
    }
    candidates.push(*ch);
  }
}

fn pick_char_from<R: Rng>(rng: &mut R, set: &[u8], params: &PasswordGeneratorCharsParam) -> u8 {
  let mut candidates = Vec::with_capacity(set.len());
  filter_set(&mut candidates, set, params);

  *candidates.choose(rng).unwrap()
}

#[cfg(test)]
mod tests {
  use super::*;
  use spectral::prelude::*;

  #[test]
  fn test_generate_chars() {
    let pw1 = generate_chars(&PasswordGeneratorCharsParam {
      num_chars: 14,
      include_uppers: false,
      include_numbers: false,
      include_symbols: false,
      require_number: false,
      require_upper: false,
      require_symbol: false,
      exclude_similar: false,
      exclude_ambiguous: false,
    });

    assert_that(&pw1.len()).is_equal_to(14);
    assert_that(&pw1.chars().all(|ch| ch.is_lowercase())).is_true();

    let pw2: String = generate_chars(&PasswordGeneratorCharsParam {
      num_chars: 20,
      include_uppers: true,
      include_numbers: false,
      include_symbols: false,
      require_number: false,
      require_upper: true,
      require_symbol: false,
      exclude_similar: false,
      exclude_ambiguous: false,
    });

    assert_that(&pw2.len()).is_equal_to(20);
    assert_that(&pw2.chars().any(|ch| ch.is_uppercase())).is_true();
  }
}
