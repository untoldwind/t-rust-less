use std::ptr::NonNull;

mod alloc;
mod memory;

pub struct SecretBytes {
    ptr: NonNull<u8>,
}

impl SecretBytes {
    pub fn new(size: usize) -> SecretBytes {
        unsafe {
            SecretBytes {
                ptr: alloc::malloc(size),
            }
        }
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        unsafe {
            alloc::free(self.ptr)
        }
    }
}
