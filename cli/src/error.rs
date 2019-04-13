use log::error;
use std::fmt;
use std::process;

pub fn exit_with_error<E>(prefix: &str, error: E)
where
  E: fmt::Display,
{
  error!("{}{}", prefix, error);

  process::exit(1)
}

pub trait ExtResult<T, E> {
  fn ok_or_exit(self, prefix: &str) -> T;
}

impl<T, E> ExtResult<T, E> for Result<T, E>
where
  E: fmt::Display,
{
  fn ok_or_exit(self, prefix: &str) -> T {
    match self {
      Ok(result) => result,
      Err(error) => {
        exit_with_error(prefix, error);
        unreachable!()
      }
    }
  }
}
