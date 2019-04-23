use std::ptr;

// -- memcmp --

/// Secure `memeq`.
#[inline(never)]
pub unsafe fn memeq(b1: *const u8, b2: *const u8, len: usize) -> bool {
  (0..len as isize)
    .map(|i| ptr::read_volatile(b1.offset(i)) ^ ptr::read_volatile(b2.offset(i)))
    .fold(0, |sum, next| sum | next)
    .eq(&0)
}

/// Secure `memcmp`.
#[inline(never)]
#[allow(dead_code)]
pub unsafe fn memcmp(b1: *const u8, b2: *const u8, len: usize) -> i32 {
  let mut res = 0;
  for i in (0..len as isize).rev() {
    let diff = i32::from(ptr::read_volatile(b1.offset(i))) - i32::from(ptr::read_volatile(b2.offset(i)));
    res = (res & (((diff - 1) & !diff) >> 8)) | diff;
  }
  ((res - 1) >> 8) + (res >> 8) + 1
}

// -- memset / memzero --

/// General `memset`.
#[cfg(feature = "nightly")]
#[cfg(any(not(apple), not(feature = "use_os")))]
#[inline(never)]
pub unsafe fn memset(s: *mut u8, c: u8, n: usize) {
  core::intrinsics::volatile_set_memory(s, c, n);
}

/// General `memset`.
#[cfg(not(feature = "nightly"))]
#[cfg(any(not(apple), not(feature = "use_os")))]
#[inline(never)]
pub unsafe fn memset(s: *mut u8, c: u8, n: usize) {
  for i in 0..n {
    ptr::write_volatile(s.add(i), c);
  }
}

/// Call `memset_s`.
#[cfg(all(apple, feature = "use_os"))]
pub unsafe fn memset(s: *mut u8, c: u8, n: usize) {
  use libc::{c_int, c_void};
  use mach_o_sys::ranlib::{errno_t, rsize_t};

  extern "C" {
    fn memset_s(s: *mut c_void, smax: rsize_t, c: c_int, n: rsize_t) -> errno_t;
  }

  if n > 0 && memset_s(s as *mut c_void, n as _, c as _, n as _) != 0 {
    std::process::abort()
  }
}

/// General `memzero`.
#[cfg(any(
  not(any(all(windows, not(target_env = "msvc")), freebsdlike, netbsdlike)),
  not(feature = "use_os")
))]
#[inline]
pub unsafe fn memzero(dest: *mut u8, n: usize) {
  memset(dest, 0, n);
}

/// Call `explicit_bzero`.
#[cfg(all(any(freebsdlike, netbsdlike), feature = "use_os"))]
pub unsafe fn memzero(dest: *mut u8, n: usize) {
  extern "C" {
    fn explicit_bzero(s: *mut libc::c_void, n: libc::size_t);
  }
  explicit_bzero(dest as *mut libc::c_void, n);
}

/// Call `SecureZeroMemory`.
#[cfg(all(windows, not(target_env = "msvc"), feature = "use_os"))]
pub unsafe fn memzero(s: *mut u8, n: usize) {
  extern "system" {
    fn RtlSecureZeroMemory(ptr: winapi::shared::ntdef::PVOID, cnt: winapi::shared::basetsd::SIZE_T);
  }
  RtlSecureZeroMemory(s as winapi::shared::ntdef::PVOID, n as winapi::shared::basetsd::SIZE_T);
}

/// Unix `mlock`.
#[cfg(unix)]
pub unsafe fn mlock(addr: *mut u8, len: usize) -> bool {
  #[cfg(target_os = "linux")]
  libc::madvise(addr as *mut ::libc::c_void, len, ::libc::MADV_DONTDUMP);

  #[cfg(freebsdlike)]
  libc::madvise(addr as *mut ::libc::c_void, len, ::libc::MADV_NOCORE);

  libc::mlock(addr as *mut ::libc::c_void, len) == 0
}

/// Windows `VirtualLock`.
#[cfg(windows)]
pub unsafe fn mlock(addr: *mut u8, len: usize) -> bool {
  winapi::um::memoryapi::VirtualLock(
    addr as ::winapi::shared::minwindef::LPVOID,
    len as ::winapi::shared::basetsd::SIZE_T,
  ) != 0
}

/// Unix `munlock`.
#[cfg(unix)]
pub unsafe fn munlock(addr: *mut u8, len: usize) -> bool {
  memzero(addr, len);

  #[cfg(target_os = "linux")]
  libc::madvise(addr as *mut ::libc::c_void, len, ::libc::MADV_DODUMP);

  #[cfg(freebsdlike)]
  libc::madvise(addr as *mut ::libc::c_void, len, ::libc::MADV_CORE);

  libc::munlock(addr as *mut ::libc::c_void, len) == 0
}

/// Windows `VirtualUnlock`.
#[cfg(windows)]
pub unsafe fn munlock(addr: *mut u8, len: usize) -> bool {
  memzero(addr, len);
  winapi::um::memoryapi::VirtualUnlock(
    addr as ::winapi::shared::minwindef::LPVOID,
    len as ::winapi::shared::basetsd::SIZE_T,
  ) != 0
}

#[cfg(test)]
mod tests {
  use std::cmp;
  use std::mem;

  use quickcheck::quickcheck;

  use super::*;

  #[test]
  fn memzero_test() {
    unsafe {
      let mut x: [usize; 16] = [1; 16];
      memzero(x.as_mut_ptr() as *mut u8, mem::size_of_val(&x));
      assert_eq!(x, [0; 16]);
      x.clone_from_slice(&[1; 16]);
      assert_eq!(x, [1; 16]);
      memzero(x[1..11].as_mut_ptr() as *mut u8, 10 * mem::size_of_val(&x[0]));
      assert_eq!(x, [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1]);
    }
  }

  #[test]
  fn memeq_test() {
    #[allow(clippy::needless_pass_by_value)]
    fn check_memeq(x: Vec<u8>, y: Vec<u8>) -> bool {
      unsafe {
        let memsec_output = memeq(x.as_ptr(), y.as_ptr(), cmp::min(x.len(), y.len()));
        let libc_output = libc::memcmp(
          x.as_ptr() as *const libc::c_void,
          y.as_ptr() as *const libc::c_void,
          cmp::min(x.len(), y.len()),
        ) == 0;
        memsec_output == libc_output
      }
    }
    quickcheck(check_memeq as fn(Vec<u8>, Vec<u8>) -> bool);
  }

  #[test]
  fn memcmp_test() {
    #[allow(clippy::needless_pass_by_value)]
    fn check_memcmp(x: Vec<u8>, y: Vec<u8>) -> bool {
      unsafe {
        let memsec_output = memcmp(x.as_ptr(), y.as_ptr(), cmp::min(x.len(), y.len()));
        let libc_output = libc::memcmp(
          x.as_ptr() as *const libc::c_void,
          y.as_ptr() as *const libc::c_void,
          cmp::min(x.len(), y.len()),
        );
        (memsec_output > 0) == (libc_output > 0)
          && (memsec_output < 0) == (libc_output < 0)
          && (memsec_output == 0) == (libc_output == 0)
      }
    }
    quickcheck(check_memcmp as fn(Vec<u8>, Vec<u8>) -> bool);
  }

  #[test]
  fn mlock_munlock_test() {
    unsafe {
      let mut x = [1; 16];

      assert!(mlock(x.as_mut_ptr(), mem::size_of_val(&x)));
      assert!(munlock(x.as_mut_ptr(), mem::size_of_val(&x)));
      assert_eq!(x, [0; 16]);
    }
  }
}
