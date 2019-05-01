use super::OTPAlgorithm;
use crate::memguard::weak::ZeroingBytes;
use byteorder::{BigEndian, ByteOrder};
use hmac::digest::generic_array::ArrayLength;
use hmac::digest::{BlockInput, FixedOutput, Input, Reset};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

#[derive(Debug)]
pub struct HOTPGenerator<'a> {
  pub algorithm: OTPAlgorithm,
  pub counter: u64,
  pub digits: u8,
  pub secret: &'a ZeroingBytes,
}

impl<'a> HOTPGenerator<'a> {
  fn calculate<D>(&mut self) -> String
  where
    D: Input + BlockInput + FixedOutput + Reset + Default + Clone,
    D::BlockSize: ArrayLength<u8> + Clone,
    D::OutputSize: ArrayLength<u8>,
  {
    let mut mac = Hmac::<D>::new_varkey(&self.secret).unwrap();
    mac.input(&self.counter.to_be_bytes());

    self.counter += 1;

    let result = mac.result();
    let digest = result.code();

    let offset: usize = (digest[digest.len() - 1] & 0xf) as usize;

    let base = BigEndian::read_u32(&digest[offset..offset + 4]) & 0x7fff_ffff;

    format!(
      "{:01$}",
      base % (10 as u32).pow(self.digits as u32),
      self.digits as usize
    )
  }

  pub fn generate(&mut self) -> (String, u64) {
    let otp = match self.algorithm {
      OTPAlgorithm::SHA1 => self.calculate::<Sha1>(),
      OTPAlgorithm::SHA256 => self.calculate::<Sha256>(),
      OTPAlgorithm::SHA512 => self.calculate::<Sha512>(),
    };
    (otp, self.counter)
  }
}