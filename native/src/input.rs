use byteorder::{ByteOrder, NativeEndian};
use serde::Deserialize;
use std::io::{Read, Result};

pub struct Input<I> {
  underlying: I,
  buffer: Vec<u8>,
}

impl<I> Input<I>
where
  I: Read,
{
  pub fn new(input: I) -> Input<I> {
    Input {
      underlying: input,
      buffer: vec![],
    }
  }

  pub fn read<'a, M>(&'a mut self) -> Result<Option<M>>
  where
    M: Deserialize<'a>,
  {
    let mut length_buffer = [0u8; 4];
    self.underlying.read_exact(&mut length_buffer)?;
    let length = NativeEndian::read_u32(&length_buffer) as usize;
    self.buffer.resize(length, 0);
    self.underlying.read_exact(&mut self.buffer)?;

    let message = serde_json::from_slice::<M>(&self.buffer);

    Ok(message.ok())
  }

  pub fn clear_buffer(&mut self) {
    for b in self.buffer.iter_mut() {
      *b = 0
    }
    self.buffer.clear()
  }
}
