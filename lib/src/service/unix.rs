use crate::api_capnp::service;
use crate::service::remote::RemoteTrustlessService;
use crate::service::ServiceResult;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::Future;
use std::path::PathBuf;
use tokio::io::AsyncRead;
use tokio::net::UnixStream;
use tokio::runtime::current_thread;

pub fn daemon_socket_path() -> PathBuf {
  dirs::runtime_dir()
    .map(|r| r.join("t-rust-less.socket"))
    .unwrap_or_else(|| {
      dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".t-rust-less-socket")
    })
}

pub fn try_remote_service() -> ServiceResult<Option<RemoteTrustlessService>> {
  let socket_path = daemon_socket_path();

  if !socket_path.exists() {
    return Ok(None);
  }

  let mut runtime = current_thread::Runtime::new()?;
  let stream = runtime.block_on(UnixStream::connect(socket_path))?;
  let (reader, writer) = stream.split();
  let network = Box::new(twoparty::VatNetwork::new(
    reader,
    std::io::BufWriter::new(writer),
    rpc_twoparty_capnp::Side::Client,
    Default::default(),
  ));
  let mut rpc_system = RpcSystem::new(network, None);
  let client: service::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
  runtime.spawn(rpc_system.map_err(|_e| ()));

  Ok(Some(RemoteTrustlessService::new(client, runtime)))
}
