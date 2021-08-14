use std::io::ErrorKind;

use crate::service::remote::RemoteTrustlessService;
use crate::service::{ServiceResult, TrustlessService};
use named_pipe::PipeClient;

pub const DAEMON_PIPE_NAME: &str = r"\\.\pipe\t-rust-less";

pub fn try_remote_service() -> ServiceResult<Option<impl TrustlessService>> {
  let stream = match PipeClient::connect(DAEMON_PIPE_NAME) {
    Ok(pipe) => pipe,
    Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
    Err(error) => return Err(error.into()),
  };

  Ok(Some(RemoteTrustlessService::new(stream)))
}
