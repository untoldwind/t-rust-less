use std::ops::{Deref, DerefMut};
use std::ptr::{copy_nonoverlapping, NonNull};
use std::slice;
use std::sync::atomic::{AtomicIsize, Ordering};

mod alloc;
mod memory;

pub struct SecretBytes {
  ptr: NonNull<u8>,
  size: usize,
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
        locks: AtomicIsize::new(0),
      }
    }
  }

  pub fn capacity(&self) -> usize {
    unsafe { alloc::capacity(self.ptr) }
  }

  pub fn borrow<'a>(&'a self) -> Ref<'a> {
    Ref { bytes: self }
  }

  pub fn borrow_mut<'a>(&'a mut self) -> RefMut<'a> {
    RefMut { bytes: self }
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
    let locks = self.locks.fetch_add(1, Ordering::Relaxed);

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
    let locks = self.locks.fetch_sub(1, Ordering::Relaxed);

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

impl From<&mut [u8]> for SecretBytes {
  fn from(bytes: &mut [u8]) -> Self {
    unsafe {
      let ptr = alloc::malloc(bytes.len());

      copy_nonoverlapping(bytes.as_ptr(), ptr.as_ptr(), bytes.len());
      memory::memzero(bytes.as_mut_ptr(), bytes.len());

      SecretBytes {
        ptr,
        size: bytes.len(),
        locks: AtomicIsize::new(0),
      }
    }
  }
}

struct Ref<'a> {
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

struct RefMut<'a> {
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

#[cfg(test)]
mod tests {
  use super::*;
    use rand::{thread_rng, Rng, ThreadRng};
    use spectral::prelude::*;

    #[test]
    fn test_borrow_read_only() {
        let mut rng = thread_rng();
        let mut source = rng.gen_iter::<u8>().take(200).collect::<Vec<u8>>();
        let expected = source.clone();

        let guarded = SecretBytes::from(source.as_mut_slice());

    }
}
