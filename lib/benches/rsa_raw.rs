#![allow(soft_unstable)]
#![feature(test)]

extern crate test;

use openssl::rsa::{Padding, Rsa};
use rand::{thread_rng, RngCore};
use rsa::{PaddingScheme, PublicKey, RSAPrivateKey};
use test::Bencher;

fn assert_slices_equal(actual: &[u8], expected: &[u8]) {
  assert!(actual == expected)
}

#[bench]
fn bench_openssl_rsa(b: &mut Bencher) {
  let mut rng = thread_rng();
  let private = Rsa::generate(4096).unwrap();
  let public = Rsa::public_key_from_der(&private.public_key_to_der().unwrap()).unwrap();

  b.iter(|| {
    let mut message = [0u8; 32];
    rng.fill_bytes(&mut message[..]);

    let mut crypled_key_buffer = vec![0u8; public.size() as usize];
    let crypted_len = public
      .public_encrypt(&message[..], &mut crypled_key_buffer, Padding::PKCS1_OAEP)
      .unwrap();

    let mut target = vec![0u8; crypted_len];

    let target_len = private
      .private_decrypt(&crypled_key_buffer[0..crypted_len], &mut target, Padding::PKCS1_OAEP)
      .unwrap();

    assert_slices_equal(&target[0..target_len], &message[..]);
  });
}

#[bench]
fn bench_rust_rsa(b: &mut Bencher) {
  let mut rng = thread_rng();
  let private = RSAPrivateKey::new(&mut rng, 4096).unwrap();
  let public = private.to_public_key();

  b.iter(|| {
    let mut message = [0u8; 32];
    rng.fill_bytes(&mut message[..]);

    let crypled_key_buffer = public
      .encrypt(&mut rng, PaddingScheme::new_oaep::<sha1::Sha1>(), &message[..])
      .unwrap();

    let target = private
      .decrypt(PaddingScheme::new_oaep::<sha1::Sha1>(), &crypled_key_buffer[..])
      .unwrap();

    assert_slices_equal(&target[..], &message[..]);
  });
}
