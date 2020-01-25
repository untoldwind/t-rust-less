use crate::error::ExtResult;
use async_std::os::unix::net::UnixListener;
use async_timer::Interval;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::channel::mpsc;
use futures::executor::LocalPool;
use futures::task::LocalSpawn;
use futures::{future, AsyncReadExt, FutureExt, StreamExt};
use log::{error, info};
use std::fs;
use std::time::Duration;
use t_rust_less_lib::service::unix::daemon_socket_path;

pub fn run_server<F, A>(handler_factory: F, check_autolock: A)
where
  F: Fn() -> capnp::capability::Client,
  A: Fn() -> (),
{
  let socket_path = daemon_socket_path();
  let socket_path_cloned = socket_path.clone();

  info!("Listening on socket {}", socket_path.to_string_lossy());

  let mut exec = LocalPool::new();
  let spawner = exec.spawner();
  let result: Result<(), Box<dyn std::error::Error>> = exec.run_until(async move {
    let prev_mask = unsafe {
      // Dirty little trick to set permissions on the socket
      libc::umask(0o177)
    };
    let socket = UnixListener::bind(&socket_path_cloned).await?;
    unsafe {
      libc::umask(prev_mask);
    }

    let mut incoming = socket.incoming();

    let handle_incoming = async move {
      while let Some(stream) = incoming.next().await {
        let (reader, writer) = stream?.split();

        let network = twoparty::VatNetwork::new(reader, writer, rpc_twoparty_capnp::Side::Server, Default::default());
        let rpc_system = RpcSystem::new(Box::new(network), Some(handler_factory()));

        spawner.spawn_local_obj(Box::pin(rpc_system.map(|_| ())).into())?;
      }

      Ok::<(), Box<dyn std::error::Error>>(())
    };

    let autolocker = Interval::platform_new(Duration::from_secs(1))
      .for_each(|_| {
        check_autolock();
        future::ready(())
      })
      .map(|_| Ok::<(), Box<dyn std::error::Error>>(()));

    let (signal_sender, signal_receiver) = mpsc::unbounded::<()>();

    ctrlc::set_handler(move || {
      signal_sender.close_channel();
    })?;

    future::select(
      Box::pin(signal_receiver.into_future()),
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
