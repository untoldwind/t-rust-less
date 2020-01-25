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
use super::memory;
use capnp::message::{AllocationStrategy, Allocator, SUGGESTED_ALLOCATION_STRATEGY, SUGGESTED_FIRST_SEGMENT_WORDS};
use capnp::Word;
use log::warn;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::ops::{Deref, DerefMut};

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
      memory::memzero(self.0.as_mut_ptr(), self.0.capacity());
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

pub trait ZeroingBytesExt {
  fn to_zeroing(self) -> ZeroingBytes;
}

impl ZeroingBytesExt for Vec<u8> {
  fn to_zeroing(self) -> ZeroingBytes {
    ZeroingBytes(self)
  }
}

impl ZeroingBytesExt for &[u8] {
  fn to_zeroing(self) -> ZeroingBytes {
    ZeroingBytes(self.to_vec())
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZeroingWords(Vec<Word>);

impl ZeroingWords {
  pub fn allocate_zeroed_vec(size: usize) -> ZeroingWords {
    ZeroingWords(Word::allocate_zeroed_vec(size))
  }

  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  pub fn len(&self) -> usize {
    self.0.len()
  }
}

impl Drop for ZeroingWords {
  fn drop(&mut self) {
    unsafe {
      memory::memzero(self.0.as_mut_ptr() as *mut u8, self.0.capacity() * 8);
    }
  }
}

impl Deref for ZeroingWords {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    unsafe { std::slice::from_raw_parts(self.0.as_ptr() as *const u8, self.0.len() * 8) }
  }
}

impl DerefMut for ZeroingWords {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { std::slice::from_raw_parts_mut(self.0.as_mut_ptr() as *mut u8, self.0.len() * 8) }
  }
}

impl From<&[u8]> for ZeroingWords {
  fn from(bytes: &[u8]) -> Self {
    if bytes.len() % 8 != 0 {
      warn!("Bytes not aligned to 8 bytes. Probably these are not the bytes you are looking for.");
    }
    let len = bytes.len() / 8;
    let mut target = ZeroingWords::allocate_zeroed_vec(len);
    unsafe {
      std::ptr::copy_nonoverlapping(bytes.as_ptr(), target.as_mut_ptr() as *mut u8, len * 8);
    }

    target
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

      memory::memzero(bytes.as_mut_ptr(), bytes.len());
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

impl AsRef<str> for ZeroingString {
  fn as_ref(&self) -> &str {
    self.0.as_ref()
  }
}

impl AsRef<[u8]> for ZeroingString {
  fn as_ref(&self) -> &[u8] {
    self.0.as_bytes()
  }
}

impl AsMut<[u8]> for ZeroingString {
  fn as_mut(&mut self) -> &mut [u8] {
    unsafe { self.0.as_bytes_mut() }
  }
}

impl Borrow<str> for ZeroingString {
  fn borrow(&self) -> &str {
    self.0.as_ref()
  }
}

pub trait ZeroingStringExt {
  fn to_zeroing(self) -> ZeroingString;
}

impl ZeroingStringExt for &str {
  fn to_zeroing(self) -> ZeroingString {
    ZeroingString(self.to_string())
  }
}

impl ZeroingStringExt for String {
  fn to_zeroing(self) -> ZeroingString {
    ZeroingString(self)
  }
}

#[derive(Debug)]
pub struct ZeroingHeapAllocator {
  owned_memory: Vec<ZeroingWords>,
  next_size: u32,
  allocation_strategy: AllocationStrategy,
}

impl ZeroingHeapAllocator {
  pub fn first_segment_words(mut self, value: u32) -> ZeroingHeapAllocator {
    self.next_size = value;
    self
  }

  pub fn allocation_strategy(mut self, value: AllocationStrategy) -> ZeroingHeapAllocator {
    self.allocation_strategy = value;
    self
  }
}

impl Default for ZeroingHeapAllocator {
  fn default() -> Self {
    ZeroingHeapAllocator {
      owned_memory: Vec::new(),
      next_size: SUGGESTED_FIRST_SEGMENT_WORDS,
      allocation_strategy: SUGGESTED_ALLOCATION_STRATEGY,
    }
  }
}

unsafe impl Allocator for ZeroingHeapAllocator {
  fn allocate_segment(&mut self, minimum_size: u32) -> (*mut u8, u32) {
    let size = ::std::cmp::max(minimum_size, self.next_size);
    let mut new_words = ZeroingWords::allocate_zeroed_vec(size as usize);
    let ptr = new_words.as_mut_ptr() as *mut u8;
    self.owned_memory.push(new_words);

    if let AllocationStrategy::GrowHeuristically = self.allocation_strategy {
      self.next_size += size;
    }
    (ptr, size as u32)
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
      assert!(zeroing.as_slice() == [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19]);
    }
    {
      let zeroing = ZeroingString::wrap("0123456789".to_string());

      assert!(zeroing.as_str() == "0123456789")
    }
    {
      let zeroing = ZeroingWords::allocate_zeroed_vec(200);

      assert_eq!(zeroing.len(), 200);

      for w in zeroing.iter() {
        assert_eq!(*w, 0);
      }
    }
  }
}
