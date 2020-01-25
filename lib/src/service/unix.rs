use crate::api_capnp::service;
use crate::service::remote::RemoteTrustlessService;
use crate::service::ServiceResult;
use async_std::os::unix::net::UnixStream;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::executor::LocalPool;
use futures::task::LocalSpawn;
use futures::{AsyncReadExt, FutureExt};
use std::path::PathBuf;

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

  let mut exec = LocalPool::new();
  let spawner = exec.spawner();
  let client: ServiceResult<service::Client> = exec.run_until(async move {
    let stream = UnixStream::connect(socket_path).await?;
    let (reader, writer) = stream.split();
    let network = Box::new(twoparty::VatNetwork::new(
      reader,
      writer,
      rpc_twoparty_capnp::Side::Client,
      Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let client: service::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    spawner.spawn_local_obj(Box::pin(rpc_system.map(|_| ())).into())?;

    Ok(client)
  });

  Ok(Some(RemoteTrustlessService::new(client?, exec)))
}
