#![allow(soft_unstable)]
#![feature(test)]

extern crate test;

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, NewAead};
use aes_gcm::Aes256Gcm;
use openssl::symm;
use rand::{thread_rng, RngCore};
use test::Bencher;

fn assert_slices_equal(actual: &[u8], expected: &[u8]) {
  assert!(actual == expected)
}

#[bench]
fn bench_openssl_aes(b: &mut Bencher) {
  let mut rng = thread_rng();

  b.iter(|| {
    let mut seal_key = [0u8; 32];
    let mut nonce = [0u8; 12];
    let message = b"Hello, secret";
    let mut tag = [0u8; 16];

    rng.fill_bytes(&mut seal_key[..]);
    rng.fill_bytes(&mut nonce[..]);

    let mut public_data = symm::encrypt_aead(
      symm::Cipher::aes_256_gcm(),
      &seal_key[..],
      Some(&nonce[..]),
      &[],
      message,
      &mut tag[..],
    )
    .unwrap();
    public_data.extend_from_slice(&tag);

    let tag_offset = public_data.len() - 16;
    let decrypted = symm::decrypt_aead(
      symm::Cipher::aes_256_gcm(),
      &seal_key[..],
      Some(&nonce[0..12]),
      &[],
      &public_data[0..tag_offset],
      &public_data[tag_offset..],
    )
    .unwrap();

    assert_slices_equal(&decrypted, message);
  });
}

#[bench]
fn bench_rust_aes(b: &mut Bencher) {
  let mut rng = thread_rng();

  b.iter(|| {
    let mut seal_key = [0u8; 32];
    let mut nonce = [0u8; 12];
    let message = b"Hello, secret";

    rng.fill_bytes(&mut seal_key[..]);
    rng.fill_bytes(&mut nonce[..]);

    let cipher = Aes256Gcm::new(GenericArray::from_slice(&seal_key[..]));
    let public_data = cipher
      .encrypt(GenericArray::from_slice(&nonce[0..12]), &message[..])
      .unwrap();
    let decrypted = cipher
      .decrypt(GenericArray::from_slice(&nonce[0..12]), &public_data[..])
      .unwrap();

    assert_slices_equal(&decrypted, message);
  });
}
