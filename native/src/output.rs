use byteorder::{ByteOrder, NativeEndian};
use serde::Serialize;
use std::io::{ErrorKind, Result, Write};
use std::sync::RwLock;

pub struct Output<O> {
  inner: RwLock<OutputInner<O>>,
}

impl<O> Output<O>
where
  O: Write,
{
  pub fn new(output: O) -> Output<O> {
    Output {
      inner: RwLock::new(OutputInner {
        underlying: output,
        buffer: vec![],
      }),
    }
  }

  pub fn send<M>(&self, message: M) -> Result<()>
  where
    M: Serialize,
  {
    match self.inner.write() {
      Ok(mut inner) => {
        inner.send(message)?;
        Ok(())
      }
      Err(_) => Err(ErrorKind::Other.into()),
    }
  }
}

struct OutputInner<O> {
  underlying: O,
  buffer: Vec<u8>,
}

impl<O> OutputInner<O>
where
  O: Write,
{
  fn clear_buffer(&mut self) {
    for b in self.buffer.iter_mut() {
      *b = 0
    }
    self.buffer.clear()
  }

  fn send<M>(&mut self, message: M) -> Result<()>
  where
    M: Serialize,
  {
    let mut length_buffer = [0u8; 4];

    serde_json::to_writer(&mut self.buffer, &message)?;
    NativeEndian::write_u32(&mut length_buffer, self.buffer.len() as u32);
    self.underlying.write_all(&length_buffer)?;
    self.underlying.write_all(&self.buffer)?;
    self.underlying.flush()?;
    self.clear_buffer();

    Ok(())
  }
}
