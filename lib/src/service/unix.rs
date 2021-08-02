use crate::service::remote::RemoteTrustlessService;
use crate::service::{ServiceResult, TrustlessService};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

pub fn daemon_socket_path() -> PathBuf {
  dirs::runtime_dir()
    .map(|r| r.join("t-rust-less.socket-v2"))
    .unwrap_or_else(|| {
      dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".t-rust-less-socket-v2")
    })
}

pub fn try_remote_service() -> ServiceResult<Option<impl TrustlessService>> {
  let socket_path = daemon_socket_path();

  if !socket_path.exists() {
    return Ok(None);
  }

  let stream = UnixStream::connect(socket_path)?;

  Ok(Some(RemoteTrustlessService::new(stream)))
}
