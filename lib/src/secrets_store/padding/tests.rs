use super::{NonZeroPadding, Padding, RandomFrontBack};
use crate::memguard::SecretBytes;
use quickcheck::quickcheck;
use rand::thread_rng;

fn assert_slices_equal(actual: &[u8], expected: &[u8]) {
  assert!(actual == expected)
}

fn common_padding_tests<T>(data: SecretBytes)
where
  T: Padding,
{
  for pad_align in &[100, 128, 200, 256, 1000, 1024] {
    let padded = T::pad_secret_data(&data.borrow(), *pad_align).unwrap();

    assert!(padded.len() % *pad_align == 0);

    let padded_borrow = padded.borrow();

    let unpadded = T::unpad_data(&padded_borrow).unwrap();

    assert_slices_equal(unpadded, &data.borrow());
  }
}

fn clean_zero_bytes(mut data: SecretBytes) -> SecretBytes {
  data.borrow_mut().iter_mut().for_each(|b| {
    if *b == 0 {
      *b = 255;
    }
  });
  data
}

#[test]
fn test_non_zero_padding() {
  let mut rng = thread_rng();

  common_padding_tests::<NonZeroPadding>(clean_zero_bytes(SecretBytes::random(&mut rng, 127)));
  common_padding_tests::<NonZeroPadding>(clean_zero_bytes(SecretBytes::random(&mut rng, 128)));
  common_padding_tests::<NonZeroPadding>(clean_zero_bytes(SecretBytes::random(&mut rng, 129)));
  common_padding_tests::<NonZeroPadding>(clean_zero_bytes(SecretBytes::random(&mut rng, 137)));
  common_padding_tests::<NonZeroPadding>(clean_zero_bytes(SecretBytes::random(&mut rng, 1234)));
  common_padding_tests::<NonZeroPadding>(clean_zero_bytes(SecretBytes::random(&mut rng, 12345)));
  common_padding_tests::<NonZeroPadding>(clean_zero_bytes(SecretBytes::random(&mut rng, 123_456)));
}

#[test]
fn test_non_zero_padding_quick() {
  #[allow(clippy::needless_pass_by_value)]
  fn check_padding(mut data: Vec<u8>) -> bool {
    for b in &mut data[..] {
      if *b == 0 {
        *b = 255;
      }
    }
    common_padding_tests::<NonZeroPadding>(SecretBytes::from(data));
    true
  }

  quickcheck(check_padding as fn(Vec<u8>) -> bool);
}

#[test]
fn test_randon_front_back_padding() {
  let mut rng = thread_rng();

  common_padding_tests::<RandomFrontBack>(SecretBytes::random(&mut rng, 127));
  common_padding_tests::<RandomFrontBack>(SecretBytes::random(&mut rng, 128));
  common_padding_tests::<RandomFrontBack>(SecretBytes::random(&mut rng, 129));
  common_padding_tests::<RandomFrontBack>(SecretBytes::random(&mut rng, 137));
  common_padding_tests::<RandomFrontBack>(SecretBytes::random(&mut rng, 1234));
  common_padding_tests::<RandomFrontBack>(SecretBytes::random(&mut rng, 12345));
  common_padding_tests::<RandomFrontBack>(SecretBytes::random(&mut rng, 123_456));
}

#[test]
fn test_randon_front_back_padding_quick() {
  #[allow(clippy::needless_pass_by_value)]
  fn check_padding(data: Vec<u8>) -> bool {
    common_padding_tests::<RandomFrontBack>(SecretBytes::from(data));
    true
  }

  quickcheck(check_padding as fn(Vec<u8>) -> bool);
}
