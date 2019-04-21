use super::alloc;
use super::memory;
use capnp::message::{AllocationStrategy, Allocator, SUGGESTED_ALLOCATION_STRATEGY, SUGGESTED_FIRST_SEGMENT_WORDS};
use capnp::Word;
use log::warn;
use std::convert::{AsMut, AsRef};
use std::ops::{Deref, DerefMut};
use std::ptr::{copy_nonoverlapping, NonNull};
use std::slice;
use std::sync::atomic::{AtomicIsize, Ordering};

/// Strictly memory protected bytes contain sensitive data.
///
/// This implementation borrows a lot of code and ideas from:
/// * https://crates.io/crates/memsec
/// * https://crates.io/crates/secrets
/// * https://download.libsodium.org/doc/memory_management
///
/// `secrets` is not good enough because it relies on libsodium which breaks the desired
/// portability of this library (at least at the time of this writing).
///
/// `memsec` is not
/// good enough because it focuses on protecting a generic type `T` which size is known at
/// compile-time. In this library we are dealing with dynamic amounts of sensitive data and
/// there is no point in securing a `Vec<u8>` via `memsec` ... all we would achieve is protecting
/// the pointer to sensitive data in unsecured space.
///
pub struct SecretWords {
  ptr: NonNull<Word>,
  size: usize,
  capacity: usize,
  locks: AtomicIsize,
}

impl SecretWords {
  pub fn with_capacity(capacity: usize) -> SecretWords {
    unsafe {
      let ptr = alloc::malloc(capacity * 8).cast();

      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretWords {
        ptr,
        size: 0,
        capacity,
        locks: AtomicIsize::new(0),
      }
    }
  }

  pub fn zeroed(size: usize) -> SecretWords {
    unsafe {
      let ptr = alloc::malloc(size * 8).cast();

      memory::memzero(ptr.as_ptr() as *mut u8, size * 8);
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretWords {
        ptr,
        size,
        capacity: size,
        locks: AtomicIsize::new(0),
      }
    }
  }

  pub fn len(&self) -> usize {
    self.size
  }

  pub fn capacity(&self) -> usize {
    self.capacity
  }

  pub fn borrow<'a>(&'a self) -> Ref<'a> {
    self.lock_read();
    Ref { words: self }
  }

  pub fn borrow_mut<'a>(&'a mut self) -> RefMut<'a> {
    self.lock_write();
    RefMut { words: self }
  }

  pub fn locks(&self) -> isize {
    self.locks.load(Ordering::Relaxed)
  }

  fn lock_read(&self) {
    let locks = self.locks.fetch_add(1, Ordering::Relaxed);

    assert!(locks >= 0);

    if locks == 0 {
      unsafe {
        alloc::mprotect(self.ptr, alloc::Prot::ReadOnly);
      }
    }
  }

  fn unlock_read(&self) {
    let locks = self.locks.fetch_sub(1, Ordering::Relaxed);

    assert!(locks > 0);

    if locks == 1 {
      unsafe {
        alloc::mprotect(self.ptr, alloc::Prot::NoAccess);
      }
    }
  }

  fn lock_write(&mut self) {
    let locks = self.locks.fetch_sub(1, Ordering::Relaxed);

    assert!(locks == 0);

    unsafe {
      alloc::mprotect(self.ptr, alloc::Prot::ReadWrite);
    }
  }

  fn unlock_write(&mut self) {
    let locks = self.locks.fetch_add(1, Ordering::Relaxed);

    assert!(locks == -1);

    unsafe {
      alloc::mprotect(self.ptr, alloc::Prot::NoAccess);
    }
  }

  /// Internal use only.
  /// This will take a write-lock and never undo it until the SecretWords are dropped.
  fn as_mut_ptr(&mut self) -> *mut Word {
    self.lock_write();

    self.ptr.as_ptr()
  }
}

unsafe impl Send for SecretWords {}

unsafe impl Sync for SecretWords {}

impl Drop for SecretWords {
  fn drop(&mut self) {
    unsafe { alloc::free(self.ptr) }
  }
}

impl Clone for SecretWords {
  fn clone(&self) -> Self {
    unsafe {
      let ptr = alloc::malloc(self.capacity * 8).cast::<Word>();

      copy_nonoverlapping(self.borrow().as_ref().as_ptr(), ptr.as_ptr(), self.capacity);
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretWords {
        ptr,
        size: self.size,
        capacity: self.capacity,
        locks: AtomicIsize::new(0),
      }
    }
  }
}

impl From<&mut [u8]> for SecretWords {
  fn from(bytes: &mut [u8]) -> Self {
    if bytes.len() % 8 != 0 {
      warn!("Bytes not aligned to 8 bytes. Probably these are not the bytes you are looking for.");
    }
    unsafe {
      let len = bytes.len() / 8;
      let ptr = alloc::malloc(len * 8).cast();

      copy_nonoverlapping(bytes.as_ptr(), ptr.as_ptr() as *mut u8, len * 8);
      memory::memzero(bytes.as_mut_ptr(), bytes.len());
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretWords {
        ptr,
        size: len,
        capacity: len,
        locks: AtomicIsize::new(0),
      }
    }
  }
}

impl From<&mut [Word]> for SecretWords {
  fn from(words: &mut [Word]) -> Self {
    unsafe {
      let ptr = alloc::malloc(words.len() * 8).cast();

      copy_nonoverlapping(words.as_ptr(), ptr.as_ptr(), words.len());
      memory::memzero(words.as_mut_ptr() as *mut u8, words.len() * 8);
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretWords {
        ptr,
        size: words.len(),
        capacity: words.len(),
        locks: AtomicIsize::new(0),
      }
    }
  }
}

pub struct Ref<'a> {
  words: &'a SecretWords,
}

impl<'a> Drop for Ref<'a> {
  fn drop(&mut self) {
    self.words.unlock_read()
  }
}

impl<'a> Deref for Ref<'a> {
  type Target = [Word];

  fn deref(&self) -> &Self::Target {
    unsafe { slice::from_raw_parts(self.words.ptr.as_ptr(), self.words.size) }
  }
}

impl<'a> AsRef<[Word]> for Ref<'a> {
  fn as_ref(&self) -> &[Word] {
    unsafe { slice::from_raw_parts(self.words.ptr.as_ptr(), self.words.size) }
  }
}

pub struct RefMut<'a> {
  words: &'a mut SecretWords,
}

impl<'a> Drop for RefMut<'a> {
  fn drop(&mut self) {
    self.words.unlock_write()
  }
}

impl<'a> Deref for RefMut<'a> {
  type Target = [Word];

  fn deref(&self) -> &Self::Target {
    unsafe { slice::from_raw_parts(self.words.ptr.as_ptr(), self.words.size) }
  }
}

impl<'a> DerefMut for RefMut<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { slice::from_raw_parts_mut(self.words.ptr.as_ptr(), self.words.size) }
  }
}

impl<'a> AsRef<[Word]> for RefMut<'a> {
  fn as_ref(&self) -> &[Word] {
    unsafe { slice::from_raw_parts(self.words.ptr.as_ptr(), self.words.size) }
  }
}

impl<'a> AsMut<[Word]> for RefMut<'a> {
  fn as_mut(&mut self) -> &mut [Word] {
    unsafe { slice::from_raw_parts_mut(self.words.ptr.as_ptr(), self.words.size) }
  }
}

pub struct SecureHHeapAllocator {
  owned_memory: Vec<SecretWords>,
  next_size: u32,
  allocation_strategy: AllocationStrategy,
}

impl SecureHHeapAllocator {
  pub fn new() -> SecureHHeapAllocator {
    SecureHHeapAllocator {
      owned_memory: Vec::new(),
      next_size: SUGGESTED_FIRST_SEGMENT_WORDS,
      allocation_strategy: SUGGESTED_ALLOCATION_STRATEGY,
    }
  }

  pub fn first_segment_words(mut self, value: u32) -> SecureHHeapAllocator {
    self.next_size = value;
    self
  }

  pub fn allocation_strategy(mut self, value: AllocationStrategy) -> SecureHHeapAllocator {
    self.allocation_strategy = value;
    self
  }
}

unsafe impl Allocator for SecureHHeapAllocator {
  fn allocate_segment(&mut self, minimum_size: u32) -> (*mut Word, u32) {
    let size = ::std::cmp::max(minimum_size, self.next_size);
    let mut new_words = SecretWords::zeroed(size as usize);
    let ptr = new_words.as_mut_ptr();
    self.owned_memory.push(new_words);

    match self.allocation_strategy {
      AllocationStrategy::GrowHeuristically => {
        self.next_size += size;
      }
      _ => {}
    }
    (ptr, size as u32)
  }
}

#[cfg(test)]
mod tests {
  use byteorder::{ByteOrder, NativeEndian};
  use rand::{distributions, thread_rng, Rng};
  use spectral::prelude::*;

  use super::*;

  fn assert_slices_equal(actual: &[Word], expected: &[Word]) {
    assert!(actual == expected)
  }

  #[test]
  fn test_borrow_read_only() {
    let mut rng = thread_rng();
    let mut source = rng
      .sample_iter::<u64, _>(&distributions::Standard)
      .filter(|w| *w != 0)
      .take(200)
      .map(|w| Word { raw_content: w })
      .collect::<Vec<Word>>();
    let expected = source.clone();

    for w in source.iter() {
      assert_that(&w.raw_content).is_not_equal_to(0);
    }

    let guarded = SecretWords::from(source.as_mut_slice());

    assert_that(&guarded.len()).is_equal_to(source.len());
    assert_that(&guarded.borrow().as_ref().len()).is_equal_to(source.len());

    for w in source.iter() {
      assert_that(&w.raw_content).is_equal_to(0);
    }

    assert_that(&guarded.locks()).is_equal_to(0);
    assert_slices_equal(&guarded.borrow(), &expected);
    assert_that(&guarded.locks()).is_equal_to(0);

    {
      let ref1 = guarded.borrow();
      let ref2 = guarded.borrow();
      let ref3 = guarded.borrow();

      assert_that(&ref1.len()).is_equal_to(200);
      assert_that(&guarded.locks()).is_equal_to(3);
      assert_slices_equal(&ref1, &expected);
      assert_slices_equal(&ref2, &expected);
      assert_slices_equal(&ref3, &expected);
    }
    assert_that(&guarded.locks()).is_equal_to(0);
  }

  #[test]
  fn test_zeroed() {
    let guarded = SecretWords::zeroed(200);

    assert_that(&guarded.len()).is_equal_to(200);
    assert_that(&guarded.capacity()).is_equal_to(200);

    {
      let ref1 = guarded.borrow();

      assert_that(&ref1.len()).is_equal_to(200);
      for w in ref1.as_ref() {
        assert_that(&w.raw_content).is_equal_to(0);
      }
    }
  }

  #[test]
  fn test_borrow_read_write() {
    let mut rng = thread_rng();
    let mut source = rng
      .sample_iter::<u64, _>(&distributions::Standard)
      .filter(|w| *w != 0)
      .take(200)
      .map(|w| Word { raw_content: w })
      .collect::<Vec<Word>>();
    let source2 = rng
      .sample_iter::<u64, _>(&distributions::Standard)
      .filter(|w| *w != 0)
      .take(200)
      .map(|w| Word { raw_content: w })
      .collect::<Vec<Word>>();
    let expected = source.clone();
    let expected2 = source2.clone();

    for w in source.iter() {
      assert_that(&w.raw_content).is_not_equal_to(0);
    }

    let mut guarded = SecretWords::from(source.as_mut_slice());

    for w in source.iter() {
      assert_that(&w.raw_content).is_equal_to(0);
    }

    assert_that(&guarded.locks()).is_equal_to(0);
    assert_slices_equal(&guarded.borrow(), &expected);

    guarded.borrow_mut().as_mut().copy_from_slice(&source2);

    assert_that(&guarded.locks()).is_equal_to(0);
    assert_slices_equal(&guarded.borrow(), &expected2);
  }

  #[test]
  fn test_from_unaligned_source() {
    let mut chunks = [0u8; 16];

    NativeEndian::write_u64(&mut chunks[0..8], 0x1234567812345678);
    NativeEndian::write_u64(&mut chunks[8..16], 0xf0e1d2c3b4a59687);

    let mut bytes1 = [0u8; 100 * 16 + 1];
    let mut bytes2 = [0u8; 100 * 16 + 3];

    for i in 0..100 {
      bytes1[i * 16 + 1..i * 16 + 1 + 16].copy_from_slice(&chunks);
      bytes2[i * 16 + 3..i * 16 + 3 + 16].copy_from_slice(&chunks);
    }

    let guarded1 = SecretWords::from(&mut bytes1[1..]);
    let guarded2 = SecretWords::from(&mut bytes2[3..]);

    for b in &bytes1[..] {
      assert_that(b).is_equal_to(0);
    }
    for b in &bytes2[..] {
      assert_that(b).is_equal_to(0);
    }

    assert_that(&guarded1.len()).is_equal_to(200);
    assert_that(&guarded2.len()).is_equal_to(200);

    for (idx, w) in guarded1.borrow().iter().enumerate() {
      if idx % 2 == 0 {
        assert_that(&w.raw_content).is_equal_to(0x1234567812345678);
      } else {
        assert_that(&w.raw_content).is_equal_to(0xf0e1d2c3b4a59687);
      }
    }
    for (idx, w) in guarded2.borrow().iter().enumerate() {
      if idx % 2 == 0 {
        assert_that(&w.raw_content).is_equal_to(0x1234567812345678);
      } else {
        assert_that(&w.raw_content).is_equal_to(0xf0e1d2c3b4a59687);
      }
    }
  }
}
