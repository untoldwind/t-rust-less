#![allow(soft_unstable)]
#![feature(test)]

extern crate test;

use rand::thread_rng;
use t_rust_less_lib::{
  memguard::SecretBytes, secrets_store::cipher::Cipher, secrets_store::cipher::OPEN_SSL_RSA_AES_GCM,
  secrets_store::cipher::RUST_RSA_AES_GCM, secrets_store::cipher::RUST_X25519CHA_CHA20POLY1305,
  secrets_store_capnp::block,
};
use test::Bencher;

fn assert_slices_equal(actual: &[u8], expected: &[u8]) {
  assert!(actual == expected)
}

fn common_data_encrypt_decrypt<T>(cipher: &T, b: &mut Bencher)
where
  T: Cipher,
{
  let mut rng = thread_rng();

  let id1 = "recipient1";
  let (public_key1, private_key1) = cipher.generate_key_pair().unwrap();

  b.iter(|| {
    let private_data = SecretBytes::random(&mut rng, 1234);
    let mut message = capnp::message::Builder::new_default();

    let mut block = message.init_root::<block::Builder>();
    let headers = block.reborrow().init_headers(1);

    let crypted_data = cipher
      .encrypt(&[(id1, public_key1.clone())], &private_data, headers.get(0))
      .unwrap();
    block.set_content(&crypted_data);

    let mut message_payload: &[u8] = &capnp::serialize::write_message_to_words(&message);

    let message_reader =
      capnp::serialize::read_message_from_flat_slice(&mut message_payload, capnp::message::ReaderOptions::new())
        .unwrap();
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
  })
}

#[bench]
fn test_openssl_rsa_aes_gcm(b: &mut Bencher) {
  common_data_encrypt_decrypt(&OPEN_SSL_RSA_AES_GCM, b);
}

#[bench]
fn test_rust_x25519_chacha20_poly1305(b: &mut Bencher) {
  common_data_encrypt_decrypt(&RUST_X25519CHA_CHA20POLY1305, b);
}

#[bench]
//#[cfg(feature = "rust_crypto")]
fn test_rust_rsa_aes_gcm(b: &mut Bencher) {
  common_data_encrypt_decrypt(&RUST_RSA_AES_GCM, b);
}
