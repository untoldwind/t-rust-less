use openssl::init;
use data_encoding::BASE64;
use std::io::prelude::*;
use super::scrypt::{scrypt, ScryptParams};
use super::scrypt_stream::decrypt;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn compat_scrypt() {
    let mut out: [u8; 64] = [0; 64];

    let params = ScryptParams::new(14, 8, 1);
    let start = SystemTime::now();
    scrypt(
        b"Test password",
        b"testsalt",
        &params,
        &mut out,
    ).unwrap();
    println!("{:?}", SystemTime::now().duration_since(start).unwrap());
    assert_eq!(
        BASE64.encode(&out),
        "cYlZa6NMNcxwrHaXJ3Zby5Xr+P3qrHFm88OK63LSnynxr7edun08Zt47qF3k91WPlHyCaId2hZfUwkfZ4A/G4Q=="
    );
}

#[test]
fn compat_scrypt_stream_decrypt() {
    let data: Vec<u8> = BASE64.decode(b"c2NyeXB0ABIAAAAIAAAAAQ+QiTNyEWthcJ/qY3sTYUS3Ytbvu6f6IsFRpyaDg5x7+62ferHxlLr3\
     XkE3t3FHstri7MHk8ECU7Q2iJwMFoLoxVZpVVDxwROccmraqFihxSu59lIOp0aeDF5wqJb2cLLaX\
     O6FkhUT36iJELbn0UIc5UT5dRdywv/c/WBrGXY3Z").unwrap();
    let mut slice: &[u8] = &data[..];
    let mut out = vec![];

    decrypt(b"12345678", &mut slice, &mut out).unwrap();
    assert!(true);
}