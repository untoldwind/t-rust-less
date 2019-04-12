use super::openssl_rsa_aes_gcm::OpenSslRsaAesGcmCipher;
use super::rust_x25519_chacha20_poly1305::RustX25519ChaCha20Poly1305Cipher;
use super::Cipher;
use crate::memguard::SecretBytes;
use crate::secrets_store_capnp::{block, KeyType};
use chacha20_poly1305_aead::decrypt;
use rand::{distributions, thread_rng, Rng, ThreadRng};
use spectral::prelude::*;

fn assert_slices_equal(actual: &[u8], expected: &[u8]) {
  assert!(actual == expected)
}

fn common_chiper_tests<T>()
where
  T: Cipher,
{
  common_private_seal_open::<T>();
  common_data_encrypt_decrypt::<T>();
}

fn common_private_seal_open<T>()
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

  assert_slices_equal(&decrypted_private.borrow(), &private_key.borrow());
}

fn common_data_encrypt_decrypt<T>()
where
  T: Cipher,
{
  let mut rng = thread_rng();
  let private_data = SecretBytes::random(&mut rng, 1234);

  let id1 = "recipient1";
  let id2 = "recipient2";
  let (public_key1, private_key1) = T::generate_key_pair().unwrap();
  let (public_key2, private_key2) = T::generate_key_pair().unwrap();

  let mut message = capnp::message::Builder::new_default();

  let mut block = message.init_root::<block::Builder>();
  let headers = block.reborrow().init_headers(1);

  let crypted_data = T::encrypt(
    &[(id1, &public_key1), (id2, &public_key2)],
    &private_data,
    headers.get(0),
  )
  .unwrap();
  block
    .init_content(crypted_data.len() as u32)
    .copy_from_slice(&crypted_data);

  let message_payload = capnp::serialize::write_message_to_words(&message);

  let message_reader =
    capnp::serialize::read_message_from_words(&message_payload, capnp::message::ReaderOptions::new()).unwrap();
  let block_reader = message_reader.get_root::<block::Reader>().unwrap();
  let cryped_content = block_reader.get_content().unwrap();

  assert_slices_equal(cryped_content, &crypted_data);

  let decrypted1 = T::decrypt(
    (id1, &private_key1),
    block_reader.get_headers().unwrap().get(0),
    cryped_content,
  )
  .unwrap();

  assert_slices_equal(&decrypted1.borrow(), &private_data.borrow());

  let decrypted2 = T::decrypt(
    (id2, &private_key2),
    block_reader.get_headers().unwrap().get(0),
    cryped_content,
  )
  .unwrap();

  assert_slices_equal(&decrypted2.borrow(), &private_data.borrow());
}

#[test]
fn test_openssl_rsa_aes_gcm_test() {
  common_chiper_tests::<OpenSslRsaAesGcmCipher>();
}

#[test]
fn test_rust_x25519_chacha20_poly1305() {
  common_chiper_tests::<RustX25519ChaCha20Poly1305Cipher>();
}
