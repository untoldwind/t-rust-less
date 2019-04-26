use log::error;
use std::fmt;
use std::process;

pub fn exit_with_error<S: AsRef<str>, E>(prefix: S, error: E)
where
  E: fmt::Display,
{
  error!("{}{}", prefix.as_ref(), error);

  process::exit(1)
}

pub trait ExtResult<T, E> {
  fn ok_or_exit<S: AsRef<str>>(self, prefix: S) -> T;
}

impl<T, E> ExtResult<T, E> for Result<T, E>
where
  E: fmt::Display,
{
  fn ok_or_exit<S: AsRef<str>>(self, prefix: S) -> T {
    match self {
      Ok(result) => result,
      Err(error) => {
        exit_with_error(prefix, error);
        unreachable!()
      }
    }
  }
}
