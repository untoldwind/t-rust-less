
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;


#[cfg(not(target_os = "linux"))]
mod fallback;
#[cfg(not(target_os = "linux"))]
pub use self::fallback::*;
