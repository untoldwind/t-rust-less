use super::openssl_rsa_aes_gcm::OpenSslRsaAesGcmCipher;
use super::rust_x25519_chacha20_poly1305::RustX25519ChaCha20Poly1305Cipher;
use super::Cipher;
use spectral::prelude::*;
use rand::{thread_rng, Rng, ThreadRng};
use crate::memguard::SecretBytes;

fn common_chiper_tests<T>()
where
  T: Cipher,
{
  let (public_key, private_key) = T::generate_key_pair().unwrap();

  assert_that(&public_key.len()).is_greater_than_or_equal_to(30);

  let mut rng = thread_rng();
  let mut seal_key_raw = rng.gen_iter::<u8>().take(T::seal_key_length()).collect::<Vec<u8>>();
  let nonce = rng.gen_iter::<u8>().take(T::seal_min_nonce_length()).collect::<Vec<u8>>();
  let seal_key = SecretBytes::from(seal_key_raw.as_mut());

  let crypted_pivate = T::seal_private_key(&seal_key, &nonce, &private_key).unwrap();
}

#[test]
fn test_openssl_rsa_aes_gcm_test() {
  common_chiper_tests::<OpenSslRsaAesGcmCipher>();
}

#[test]
fn test_rust_x25519_chacha20_poly1305() {
  common_chiper_tests::<RustX25519ChaCha20Poly1305Cipher>();
}