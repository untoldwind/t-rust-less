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
    ptr::write_volatile(s.offset(i as isize), c);
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

  ::libc::mlock(addr as *mut ::libc::c_void, len) == 0
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
