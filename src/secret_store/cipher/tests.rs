use super::openssl_rsa_aes_gcm::OpenSslRsaAesGcmCipher;
use super::rust_x25519_chacha20_poly1305::RustX25519ChaCha20Poly1305Cipher;
use super::Cipher;
use crate::memguard::SecretBytes;
use rand::{distributions, thread_rng, Rng, ThreadRng};
use spectral::prelude::*;

fn common_chiper_tests<T>()
where
  T: Cipher,
{
  let (public_key, private_key) = T::generate_key_pair().unwrap();

  assert_that(&public_key.len()).is_greater_than_or_equal_to(30);

  let mut rng = thread_rng();
  let mut seal_key_raw = rng
    .sample_iter(&distributions::Standard)
    .take(T::seal_key_length())
    .collect::<Vec<u8>>();
  let nonce = rng
    .sample_iter(&distributions::Standard)
    .take(T::seal_min_nonce_length())
    .collect::<Vec<u8>>();
  let seal_key = SecretBytes::from(seal_key_raw.as_mut());

  let crypted_private = T::seal_private_key(&seal_key, &nonce, &private_key).unwrap();
  let decrypted_private = T::open_private_key(&seal_key, &nonce, &crypted_private).unwrap();
}

#[test]
fn test_openssl_rsa_aes_gcm_test() {
  common_chiper_tests::<OpenSslRsaAesGcmCipher>();
}

#[test]
fn test_rust_x25519_chacha20_poly1305() {
  common_chiper_tests::<RustX25519ChaCha20Poly1305Cipher>();
}
