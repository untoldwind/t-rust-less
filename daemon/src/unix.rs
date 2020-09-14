use crate::error::ExtResult;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{future, AsyncReadExt, FutureExt, StreamExt};
use log::{error, info};
use std::fs;
use std::time::Duration;
use t_rust_less_lib::service::unix::daemon_socket_path;
use tokio::net::UnixListener;
use tokio::runtime::Builder;
use tokio::signal;
use tokio::signal::unix::{signal, SignalKind};
use tokio::task::{self, LocalSet};
use tokio::time::interval;

pub fn run_server<F, A>(handler_factory: F, check_autolock: A)
where
  F: Fn() -> capnp::capability::Client,
  A: Fn(),
{
  let socket_path = daemon_socket_path();
  let socket_path_cloned = socket_path.clone();

  info!("Listening on socket {}", socket_path.to_string_lossy());

  let mut rt = Builder::new().basic_scheduler().enable_all().build().unwrap();
  let local_set = LocalSet::new();
  let result: Result<(), Box<dyn std::error::Error>> = local_set.block_on(&mut rt, async move {
    let prev_mask = unsafe {
      // Dirty little trick to set permissions on the socket
      libc::umask(0o177)
    };
    let mut socket = UnixListener::bind(&socket_path_cloned)?;
    unsafe {
      libc::umask(prev_mask);
    }

    let handle_incoming = async move {
      while let Ok((stream, _)) = socket.accept().await {
        let (reader, writer) = tokio_util::compat::Tokio02AsyncReadCompatExt::compat(stream).split();

        let network = twoparty::VatNetwork::new(reader, writer, rpc_twoparty_capnp::Side::Server, Default::default());
        let rpc_system = RpcSystem::new(Box::new(network), Some(handler_factory()));

        task::spawn_local(Box::pin(rpc_system.map(|_| ())));
      }

      Ok::<(), Box<dyn std::error::Error>>(())
    };

    let autolocker = interval(Duration::from_secs(1))
      .for_each(|_| {
        check_autolock();
        future::ready(())
      })
      .map(|_| Ok::<(), Box<dyn std::error::Error>>(()));

    future::select(
      future::select(
        Box::pin(signal::ctrl_c().map(|_| Ok::<(), Box<dyn std::error::Error>>(()))),
        Box::pin(
          signal(SignalKind::terminate())?
            .recv()
            .map(|_| Ok::<(), Box<dyn std::error::Error>>(())),
        ),
      ),
      Box::pin(future::try_join(handle_incoming, autolocker)),
    )
    .await;
    Ok(())
  });

  result.ok_or_exit(format!("Listen to {} failed", socket_path.to_string_lossy()));

  info!("Cleaning up");
  if let Err(error) = fs::remove_file(&socket_path) {
    error!("Cleanup of {} failed: {}", socket_path.to_string_lossy(), error)
  }
}
