use std::alloc::{alloc, dealloc, Layout};
use std::mem;
use std::ptr::{self, NonNull};
use std::sync::Once;

use rand::rngs::OsRng;
use rand::RngCore;

use super::memory;

const CANARY_SIZE: usize = 16;
static ALLOC_INIT: Once = Once::new();
static mut PAGE_SIZE: usize = 0;
static mut PAGE_MASK: usize = 0;
static mut CANARY: [u8; CANARY_SIZE] = [0; CANARY_SIZE];

#[inline]
unsafe fn alloc_init() {
  #[cfg(unix)]
  {
    PAGE_SIZE = ::libc::sysconf(::libc::_SC_PAGESIZE) as usize;
  }

  #[cfg(windows)]
  {
    let mut si = mem::uninitialized();
    ::winapi::um::sysinfoapi::GetSystemInfo(&mut si);
    PAGE_SIZE = si.dwPageSize as usize;
  }

  if PAGE_SIZE < CANARY_SIZE || PAGE_SIZE < mem::size_of::<usize>() {
    panic!("OS page to small to operate with")
  }

  PAGE_MASK = PAGE_SIZE - 1;

  OsRng.fill_bytes(&mut CANARY);
}

#[inline]
pub unsafe fn alloc_aligned(size: usize) -> NonNull<u8> {
  let layout = Layout::from_size_align_unchecked(size, PAGE_SIZE);
  NonNull::new_unchecked(alloc(layout))
}

#[inline]
pub unsafe fn free_aligned(memptr: *mut u8, size: usize) {
  let layout = Layout::from_size_align_unchecked(size, PAGE_SIZE);
  dealloc(memptr, layout);
}

/// Prot enum.
#[cfg(unix)]
#[allow(non_snake_case, non_upper_case_globals)]
#[allow(dead_code)]
pub mod Prot {
  pub use libc::c_int as Ty;

  pub const NoAccess: Ty = libc::PROT_NONE;
  pub const ReadOnly: Ty = libc::PROT_READ;
  pub const WriteOnly: Ty = libc::PROT_WRITE;
  pub const ReadWrite: Ty = (libc::PROT_READ | libc::PROT_WRITE);
  pub const Execute: Ty = libc::PROT_EXEC;
  pub const ReadExec: Ty = (libc::PROT_READ | libc::PROT_EXEC);
  pub const WriteExec: Ty = (libc::PROT_WRITE | libc::PROT_EXEC);
  pub const ReadWriteExec: Ty = (libc::PROT_READ | libc::PROT_WRITE | libc::PROT_EXEC);
}

/// Prot enum.
#[cfg(windows)]
#[allow(non_snake_case, non_upper_case_globals)]
#[allow(dead_code)]
pub mod Prot {
  pub use winapi::shared::minwindef::DWORD as Ty;

  pub const NoAccess: Ty = winapi::um::winnt::PAGE_NOACCESS;
  pub const ReadOnly: Ty = winapi::um::winnt::PAGE_READONLY;
  pub const ReadWrite: Ty = winapi::um::winnt::PAGE_READWRITE;
  pub const WriteCopy: Ty = winapi::um::winnt::PAGE_WRITECOPY;
  pub const Execute: Ty = winapi::um::winnt::PAGE_EXECUTE;
  pub const ReadExec: Ty = winapi::um::winnt::PAGE_EXECUTE_READ;
  pub const ReadWriteExec: Ty = winapi::um::winnt::PAGE_EXECUTE_READWRITE;
  pub const WriteCopyExec: Ty = winapi::um::winnt::PAGE_EXECUTE_WRITECOPY;
  pub const Guard: Ty = winapi::um::winnt::PAGE_GUARD;
  pub const NoCache: Ty = winapi::um::winnt::PAGE_NOCACHE;
  pub const WriteCombine: Ty = winapi::um::winnt::PAGE_WRITECOMBINE;
  pub const RevertToFileMap: Ty = winapi::um::winnt::PAGE_REVERT_TO_FILE_MAP;
  pub const TargetsInvalid: Ty = winapi::um::winnt::PAGE_TARGETS_INVALID;
  pub const TargetsNoUpdate: Ty = winapi::um::winnt::PAGE_TARGETS_NO_UPDATE;
}

/// Unix `mprotect`.
#[cfg(unix)]
#[inline]
pub unsafe fn _mprotect(ptr: *mut u8, len: usize, prot: Prot::Ty) -> bool {
  libc::mprotect(ptr as *mut libc::c_void, len, prot as libc::c_int) == 0
}

/// Windows `VirtualProtect`.
#[cfg(windows)]
#[inline]
pub unsafe fn _mprotect(ptr: *mut u8, len: usize, prot: Prot::Ty) -> bool {
  let mut old = mem::uninitialized();
  winapi::um::memoryapi::VirtualProtect(
    ptr as winapi::shared::minwindef::LPVOID,
    len as winapi::shared::basetsd::SIZE_T,
    prot as winapi::shared::minwindef::DWORD,
    &mut old as winapi::shared::minwindef::PDWORD,
  ) != 0
}

/// Secure `mprotect`.
#[cfg(any(unix, windows))]
#[allow(clippy::cast_ptr_alignment)]
pub unsafe fn mprotect<T>(memptr: NonNull<T>, prot: Prot::Ty) -> bool {
  let memptr = memptr.as_ptr() as *mut u8;

  let unprotected_ptr = unprotected_ptr_from_user_ptr(memptr);
  let base_ptr = unprotected_ptr.offset(-(PAGE_SIZE as isize * 2));
  let unprotected_size = ptr::read(base_ptr as *const usize);
  _mprotect(unprotected_ptr, unprotected_size, prot)
}

#[inline]
unsafe fn page_round(size: usize) -> usize {
  (size + PAGE_MASK) & !PAGE_MASK
}

#[inline]
unsafe fn unprotected_ptr_from_user_ptr(memptr: *const u8) -> *mut u8 {
  let canary_ptr = memptr.offset(-(CANARY_SIZE as isize));
  let unprotected_ptr_u = canary_ptr as usize & !PAGE_MASK;
  if unprotected_ptr_u <= PAGE_SIZE * 2 {
    panic!("Wrong page allignment")
  }
  unprotected_ptr_u as *mut u8
}

#[allow(clippy::cast_ptr_alignment)]
pub unsafe fn malloc(size: usize) -> NonNull<u8> {
  ALLOC_INIT.call_once(|| alloc_init());

  if size >= core::usize::MAX - PAGE_SIZE * 4 {
    panic!("malloc fail: not enough memory")
  }

  // aligned alloc ptr
  let size_with_canary = CANARY_SIZE + size;
  let unprotected_size = page_round(size_with_canary);
  let total_size = PAGE_SIZE + PAGE_SIZE + unprotected_size + PAGE_SIZE;
  let base_ptr = alloc_aligned(total_size).as_ptr();
  let unprotected_ptr = base_ptr.offset(PAGE_SIZE as isize * 2);

  // mprotect ptr
  _mprotect(base_ptr.add(PAGE_SIZE), PAGE_SIZE, Prot::NoAccess);
  _mprotect(unprotected_ptr.add(unprotected_size), PAGE_SIZE, Prot::NoAccess);
  memory::mlock(unprotected_ptr, unprotected_size);

  let canary_ptr = unprotected_ptr.offset(unprotected_size as isize - size_with_canary as isize);
  let user_ptr = canary_ptr.add(CANARY_SIZE);
  ptr::copy_nonoverlapping(CANARY.as_ptr(), canary_ptr, CANARY_SIZE);
  ptr::write_unaligned(base_ptr as *mut usize, unprotected_size);
  _mprotect(base_ptr, PAGE_SIZE, Prot::ReadOnly);

  assert_eq!(unprotected_ptr_from_user_ptr(user_ptr), unprotected_ptr);

  NonNull::new_unchecked(user_ptr as *mut u8)
}

/// Secure `free`.
#[allow(clippy::cast_ptr_alignment)]
pub unsafe fn free<T>(memptr: NonNull<T>) {
  let memptr = memptr.as_ptr() as *mut u8;

  // get unprotected ptr
  let canary_ptr = memptr.offset(-(CANARY_SIZE as isize));
  let unprotected_ptr = unprotected_ptr_from_user_ptr(memptr);
  let base_ptr = unprotected_ptr.offset(-(PAGE_SIZE as isize * 2));
  let unprotected_size = ptr::read(base_ptr as *const usize);
  let total_size = PAGE_SIZE + PAGE_SIZE + unprotected_size + PAGE_SIZE;
  _mprotect(base_ptr, total_size, Prot::ReadWrite);

  // check
  assert!(memory::memeq(canary_ptr as *const u8, CANARY.as_ptr(), CANARY_SIZE));

  // free
  memory::memzero(unprotected_ptr, unprotected_size);
  memory::munlock(unprotected_ptr, unprotected_size);

  free_aligned(base_ptr, total_size);
}

#[cfg(test)]
mod tests {
  use spectral::prelude::*;

  use super::super::memory;
  use super::*;

  #[test]
  fn test_alloc_free() {
    unsafe {
      let ptr = malloc(137 * 97);

      memory::memzero(ptr.as_ptr(), 137 * 97);

      let as_slice = std::slice::from_raw_parts(ptr.as_ptr(), 137 * 97);

      assert_that(&as_slice.len()).is_equal_to(137 * 97);

      for b in as_slice {
        assert_that(b).is_equal_to(0u8);
      }

      free(ptr);
    }
  }

  #[test]
  fn test_mprotect() {
    unsafe {
      let ptr = malloc(137);

      mprotect(ptr, Prot::WriteOnly);

      for idx in 0..137 {
        ptr::write(ptr.as_ptr().offset(idx as isize), idx as u8);
      }

      mprotect(ptr, Prot::ReadOnly);

      for idx in 0..137 {
        assert_that(&ptr::read(ptr.as_ptr().offset(idx as isize))).is_equal_to(idx as u8);
      }

      free(ptr);

      // This is actually quite illegal, just testing that memory has been zeroed on free
      for idx in 0..137 {
        assert_that(&ptr::read(ptr.as_ptr().offset(idx as isize))).is_equal_to(0);
      }
    }
  }
}
