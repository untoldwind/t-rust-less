use std::convert::{AsMut, AsRef};
use std::io;
use std::ops::{Deref, DerefMut};
use std::ptr::{copy_nonoverlapping, NonNull};
use std::slice;
use std::sync::atomic::{AtomicIsize, Ordering};

use rand::{CryptoRng, RngCore};

mod alloc;
pub mod memory;
pub mod weak;

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
pub struct SecretBytes {
  ptr: NonNull<u8>,
  size: usize,
  capacity: usize,
  locks: AtomicIsize,
}

impl SecretBytes {
  pub fn with_capacity(capacity: usize) -> SecretBytes {
    unsafe {
      let ptr = alloc::malloc(capacity);

      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretBytes {
        ptr,
        size: 0,
        capacity,
        locks: AtomicIsize::new(0),
      }
    }
  }

  pub fn zeroed(size: usize) -> SecretBytes {
    unsafe {
      let ptr = alloc::malloc(size);

      memory::memzero(ptr.as_ptr(), size);
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretBytes {
        ptr,
        size,
        capacity: size,
        locks: AtomicIsize::new(0),
      }
    }
  }

  pub fn random<T>(rng: &mut T, size: usize) -> SecretBytes
  where
    T: RngCore + CryptoRng,
  {
    unsafe {
      let ptr = alloc::malloc(size);

      rng.fill_bytes(slice::from_raw_parts_mut(ptr.as_ptr(), size));
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretBytes {
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
    Ref { bytes: self }
  }

  pub fn borrow_mut<'a>(&'a mut self) -> RefMut<'a> {
    self.lock_write();
    RefMut { bytes: self }
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
}

impl Drop for SecretBytes {
  fn drop(&mut self) {
    unsafe { alloc::free(self.ptr) }
  }
}

impl Clone for SecretBytes {
  fn clone(&self) -> Self {
    unsafe {
      let ptr = alloc::malloc(self.capacity);

      copy_nonoverlapping(self.borrow().as_ref().as_ptr(), ptr.as_ptr(), self.capacity);
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretBytes {
        ptr,
        size: self.size,
        capacity: self.capacity,
        locks: AtomicIsize::new(0),
      }
    }
  }
}

impl From<&mut [u8]> for SecretBytes {
  fn from(bytes: &mut [u8]) -> Self {
    unsafe {
      let ptr = alloc::malloc(bytes.len());

      copy_nonoverlapping(bytes.as_ptr(), ptr.as_ptr(), bytes.len());
      memory::memzero(bytes.as_mut_ptr(), bytes.len());
      alloc::mprotect(ptr, alloc::Prot::NoAccess);

      SecretBytes {
        ptr,
        size: bytes.len(),
        capacity: bytes.len(),
        locks: AtomicIsize::new(0),
      }
    }
  }
}

pub struct Ref<'a> {
  bytes: &'a SecretBytes,
}

impl<'a> Drop for Ref<'a> {
  fn drop(&mut self) {
    self.bytes.unlock_read()
  }
}

impl<'a> Deref for Ref<'a> {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    unsafe { slice::from_raw_parts(self.bytes.ptr.as_ptr(), self.bytes.size) }
  }
}

impl<'a> AsRef<[u8]> for Ref<'a> {
  fn as_ref(&self) -> &[u8] {
    unsafe { slice::from_raw_parts(self.bytes.ptr.as_ptr(), self.bytes.size) }
  }
}

pub struct RefMut<'a> {
  bytes: &'a mut SecretBytes,
}

impl<'a> Drop for RefMut<'a> {
  fn drop(&mut self) {
    self.bytes.unlock_write()
  }
}

impl<'a> Deref for RefMut<'a> {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    unsafe { slice::from_raw_parts(self.bytes.ptr.as_ptr(), self.bytes.size) }
  }
}

impl<'a> DerefMut for RefMut<'a> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { slice::from_raw_parts_mut(self.bytes.ptr.as_ptr(), self.bytes.size) }
  }
}

impl<'a> AsRef<[u8]> for RefMut<'a> {
  fn as_ref(&self) -> &[u8] {
    unsafe { slice::from_raw_parts(self.bytes.ptr.as_ptr(), self.bytes.size) }
  }
}

impl<'a> AsMut<[u8]> for RefMut<'a> {
  fn as_mut(&mut self) -> &mut [u8] {
    unsafe { slice::from_raw_parts_mut(self.bytes.ptr.as_ptr(), self.bytes.size) }
  }
}

impl<'a> io::Write for RefMut<'a> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    let available = self.bytes.capacity() - self.bytes.size;

    if available == 0 {
      return Err(io::ErrorKind::WriteZero.into());
    }
    let transfer = available.min(buf.len());

    unsafe {
      copy_nonoverlapping(
        buf.as_ptr(),
        self.bytes.ptr.as_ptr().offset(self.bytes.size as isize),
        transfer,
      );
    }
    self.bytes.size += transfer;

    Ok(transfer)
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use core::borrow::Borrow;

  use rand::{distributions, thread_rng, Rng};
  use spectral::prelude::*;

  use super::*;

  fn assert_slices_equal(actual: &[u8], expected: &[u8]) {
    assert!(actual == expected)
  }

  #[test]
  fn test_borrow_read_only() {
    let mut rng = thread_rng();
    let mut source = rng
      .sample_iter(&distributions::Standard)
      .filter(|b| *b != 0)
      .take(200)
      .collect::<Vec<u8>>();
    let expected = source.clone();

    for b in source.iter() {
      assert_that(b).is_not_equal_to(0);
    }

    let guarded = SecretBytes::from(source.as_mut_slice());

    assert_that(&guarded.len()).is_equal_to(source.len());
    assert_that(&guarded.borrow().as_ref().len()).is_equal_to(source.len());

    for b in source.iter() {
      assert_that(b).is_equal_to(0);
    }

    assert_that(&guarded.locks()).is_equal_to(0);
    assert_slices_equal(&guarded.borrow(), &expected);
    assert_that(&guarded.locks()).is_equal_to(0);

    {
      let ref1 = guarded.borrow();
      let ref2 = guarded.borrow();
      let ref3 = guarded.borrow();

      assert_that(&guarded.locks()).is_equal_to(3);
      assert_slices_equal(&ref1, &expected);
      assert_slices_equal(&ref2, &expected);
      assert_slices_equal(&ref3, &expected);
    }
    assert_that(&guarded.locks()).is_equal_to(0);
  }

  #[test]
  fn test_borrow_read_write() {
    let mut rng = thread_rng();
    let mut source = rng
      .sample_iter(&distributions::Standard)
      .filter(|b| *b != 0)
      .take(200)
      .collect::<Vec<u8>>();
    let source2 = rng
      .sample_iter(&distributions::Standard)
      .filter(|b| *b != 0)
      .take(200)
      .collect::<Vec<u8>>();
    let expected = source.clone();
    let expected2 = source2.clone();

    for b in source.iter() {
      assert_that(b).is_not_equal_to(0);
    }

    let mut guarded = SecretBytes::from(source.as_mut_slice());

    for b in source.iter() {
      assert_that(b).is_equal_to(0);
    }

    assert_that(&guarded.locks()).is_equal_to(0);
    assert_slices_equal(&guarded.borrow(), &expected);

    guarded.borrow_mut().as_mut().copy_from_slice(&source2);

    assert_that(&guarded.locks()).is_equal_to(0);
    assert_slices_equal(&guarded.borrow(), &expected2);
  }

  #[test]
  fn test_init_strong_random() {
    let mut rng = thread_rng();
    let random = SecretBytes::random(&mut rng, 32);

    assert_that(&random.len()).is_equal_to(32);
    assert_that(&random.borrow().as_ref().len()).is_equal_to(32);
  }
}
