use super::Cipher;
use super::openssl_rsa_aes_gcm::OpenSslRsaAesGcmCipher;
use spectral::prelude::*;

fn common_chiper_tests<T>()
where
  T: Cipher,
{
  let (public_key, private_key) = T::generate_key_pair().unwrap();

  assert_that(&public_key.len()).is_greater_than_or_equal_to(300);
}

#[test]
fn openssl_rsa_aes_gcm_test() {
  common_chiper_tests::<OpenSslRsaAesGcmCipher>();
}