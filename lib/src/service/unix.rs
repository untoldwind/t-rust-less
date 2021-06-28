use crate::api_capnp::service;
use crate::service::remote::RemoteTrustlessService;
use crate::service::ServiceResult;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{AsyncReadExt, FutureExt};
use std::path::PathBuf;
use tokio::net::UnixStream;
use tokio::runtime::Builder;
use tokio::task::{self, LocalSet};

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

  let mut rt = Builder::new_current_thread().enable_all().build().unwrap();
  let local_set = LocalSet::new();
  let client: ServiceResult<service::Client> = local_set.block_on(&mut rt, async move {
    let stream = UnixStream::connect(socket_path).await?;
    let (reader, writer) = tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
    let network = Box::new(twoparty::VatNetwork::new(
      reader,
      writer,
      rpc_twoparty_capnp::Side::Client,
      Default::default(),
    ));
    let mut rpc_system = RpcSystem::new(network, None);
    let client: service::Client = rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
    task::spawn_local(Box::pin(rpc_system.map(|_| ())));

    Ok(client)
  });

  Ok(Some(RemoteTrustlessService::new(client?, rt, local_set)))
}
