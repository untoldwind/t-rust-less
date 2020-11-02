use rand::{distributions, thread_rng, Rng};
use spectral::prelude::*;

use crate::memguard::SecretBytes;
use crate::secrets_store::cipher::{OPEN_SSL_RSA_AES_GCM, RUST_RSA_AES_GCM, RUST_X25519CHA_CHA20POLY1305};
use crate::secrets_store_capnp::block;

use super::Cipher;

fn assert_slices_equal(actual: &[u8], expected: &[u8]) {
  assert!(actual == expected)
}

fn common_chiper_tests<T>(cipher: &T)
where
  T: Cipher,
{
  common_private_seal_open(cipher);
  common_data_encrypt_decrypt(cipher);
}

fn common_private_seal_open<T>(cipher: &T)
where
  T: Cipher,
{
  let (public_key, private_key) = cipher.generate_key_pair().unwrap();

  assert_that(&public_key.len()).is_greater_than_or_equal_to(30);

  let mut rng = thread_rng();
  let nonce = rng
    .sample_iter(&distributions::Standard)
    .take(cipher.seal_min_nonce_length())
    .collect::<Vec<u8>>();
  let seal_key = SecretBytes::random(&mut rng, cipher.seal_key_length());

  let crypted_private = cipher.seal_private_key(&seal_key, &nonce, &private_key).unwrap();
  let decrypted_private = cipher.open_private_key(&seal_key, &nonce, &crypted_private).unwrap();

  assert_slices_equal(&decrypted_private.borrow(), &private_key.borrow());
}

fn common_data_encrypt_decrypt<T>(cipher: &T)
where
  T: Cipher,
{
  let mut rng = thread_rng();
  let private_data = SecretBytes::random(&mut rng, 1234);

  let id1 = "recipient1";
  let id2 = "recipient2";
  let (public_key1, private_key1) = cipher.generate_key_pair().unwrap();
  let (public_key2, private_key2) = cipher.generate_key_pair().unwrap();

  let mut message = capnp::message::Builder::new_default();

  let mut block = message.init_root::<block::Builder>();
  let headers = block.reborrow().init_headers(1);

  let crypted_data = cipher
    .encrypt(&[(id1, public_key1), (id2, public_key2)], &private_data, headers.get(0))
    .unwrap();
  block.set_content(&crypted_data);

  let mut message_payload: &[u8] = &capnp::serialize::write_message_to_words(&message);

  let message_reader =
    capnp::serialize::read_message_from_flat_slice(&mut message_payload, capnp::message::ReaderOptions::new()).unwrap();
  let block_reader = message_reader.get_root::<block::Reader>().unwrap();
  let cryped_content = block_reader.get_content().unwrap();

  assert_slices_equal(cryped_content, &crypted_data);

  let decrypted1 = cipher
    .decrypt(
      (id1, &private_key1),
      block_reader.get_headers().unwrap().get(0),
      cryped_content,
    )
    .unwrap();

  assert_slices_equal(&decrypted1.borrow(), &private_data.borrow());

  let decrypted2 = cipher
    .decrypt(
      (id2, &private_key2),
      block_reader.get_headers().unwrap().get(0),
      cryped_content,
    )
    .unwrap();

  assert_slices_equal(&decrypted2.borrow(), &private_data.borrow());
}

#[test]
fn test_openssl_rsa_aes_gcm() {
  common_chiper_tests(&OPEN_SSL_RSA_AES_GCM);
}

#[test]
fn test_rust_x25519_chacha20_poly1305() {
  common_chiper_tests(&RUST_X25519CHA_CHA20POLY1305);
}

#[test]
//#[cfg(feature = "rust_crypto")]
fn test_rust_rsa_aes_gcm() {
  common_chiper_tests(&RUST_RSA_AES_GCM);
}
