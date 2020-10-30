use std::io;
use std::ops;
use zeroize::Zeroize;

pub struct ZeroizeBytesBuffer(Vec<u8>);

impl ZeroizeBytesBuffer {
  pub fn with_capacity(initial_capacity: usize) -> ZeroizeBytesBuffer {
    ZeroizeBytesBuffer(Vec::with_capacity(initial_capacity))
  }
}

impl Zeroize for ZeroizeBytesBuffer {
  fn zeroize(&mut self) {
    self.0.zeroize();
  }
}

impl Drop for ZeroizeBytesBuffer {
  fn drop(&mut self) {
    self.0.zeroize()
  }
}

impl ops::Deref for ZeroizeBytesBuffer {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    self.0.as_ref()
  }
}

impl io::Write for ZeroizeBytesBuffer {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    let avalable = self.0.capacity() - self.0.len();

    if buf.len() < avalable {
      // This should not require any reallocation
      self.0.extend_from_slice(buf);
    } else {
      // To be on the save side with create copy with larger capacity and zero out the old
      let next_size = 2 * (self.0.capacity() + buf.len());
      let mut next_buffer = Vec::with_capacity(next_size);

      next_buffer.extend_from_slice(&self.0);
      next_buffer.extend_from_slice(buf);

      self.0.zeroize();
      self.0 = next_buffer;
    }

    Ok(buf.len())
  }

  fn flush(&mut self) -> io::Result<()> {
    Ok(())
  }
}
