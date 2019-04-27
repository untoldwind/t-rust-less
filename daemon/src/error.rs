use log::error;
use std::{fmt, process};

pub fn exit_with_error<S: AsRef<str>, E>(message: S, error: E)
where
  E: fmt::Display,
{
  error!("Fatal {}: {}", message.as_ref(), error);

  process::exit(1)
}

pub trait ExtResult<T, E> {
  fn ok_or_exit<S: AsRef<str>>(self, message: S) -> T;
}

impl<T, E> ExtResult<T, E> for Result<T, E>
where
  E: fmt::Display,
{
  fn ok_or_exit<S: AsRef<str>>(self, message: S) -> T {
    match self {
      Ok(result) => result,
      Err(error) => {
        exit_with_error(message, error);
        unreachable!()
      }
    }
  }
}
