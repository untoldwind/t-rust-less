//! # Zeroing wrappers
//!
//! These are weak variants of SecretBytes. Basically they are just wrappers around Vec<u8> and
//! String with an addition Drop zeroing their contents. These are mostly used inside the API
//! structs and do not provide a 100% guaranty that no sensitive data remains in memory.
//!
//! After all though: Data send and received via the API is processed by different clients and
//! most likely copy-pasted by to displayed to the user ... so there is no 100% guaranty anyway
//! and it would be a waste of effort providing a super-tight security.
//!
use serde_derive::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use std::ptr;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ZeroingBytes(Vec<u8>);

impl ZeroingBytes {
  pub fn wrap(v: Vec<u8>) -> ZeroingBytes {
    ZeroingBytes(v)
  }
  pub fn with_capacity(capacity: usize) -> ZeroingBytes {
    ZeroingBytes(Vec::with_capacity(capacity))
  }
}

impl Drop for ZeroingBytes {
  fn drop(&mut self) {
    unsafe {
      ptr::write_bytes(self.0.as_mut_ptr(), 0, self.0.capacity());
    }
  }
}

impl Deref for ZeroingBytes {
  type Target = Vec<u8>;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for ZeroingBytes {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ZeroingString(String);

impl ZeroingString {
  pub fn wrap(s: String) -> ZeroingString {
    ZeroingString(s)
  }
}

impl Drop for ZeroingString {
  fn drop(&mut self) {
    unsafe {
      let bytes = self.0.as_bytes_mut();

      ptr::write_bytes(bytes.as_mut_ptr(), 0, bytes.len());
    }
  }
}

impl Deref for ZeroingString {
  type Target = String;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for ZeroingString {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn test_zeroing_drop() {
    {
      let mut zeroing = ZeroingBytes::with_capacity(20);

      zeroing.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
      assert!(zeroing.as_slice() == &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
    }
    {
      let zeroing = ZeroingString::wrap("0123456789".to_string());

      assert!(zeroing.as_str() == "0123456789")
    }
  }
}
