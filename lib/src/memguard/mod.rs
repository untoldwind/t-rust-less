mod alloc;
mod bytes;
pub mod memory;
pub mod weak;
mod words;

pub use self::bytes::SecretBytes;
pub use self::words::{SecretWords, SecureHHeapAllocator};
