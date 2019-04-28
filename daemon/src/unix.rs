use crate::error::ExtResult;
use capnp_rpc::{rpc_twoparty_capnp, twoparty, RpcSystem};
use futures::{Future, Stream};
use log::{error, info};
use std::fs;
use std::time::Duration;
use t_rust_less_lib::service::unix::daemon_socket_path;
use tokio::io::AsyncRead;
use tokio::runtime::current_thread;
use tokio::timer::Interval;
use tokio_signal::unix::{Signal, SIGINT, SIGTERM};
use tokio_uds::UnixListener;

pub fn run_server<F, A>(handler_factory: F, check_autolock: A)
where
  F: Fn() -> capnp::capability::Client,
  A: Fn() -> (),
{
  let socket_path = daemon_socket_path();

  info!("Listening on socket {}", socket_path.to_string_lossy());

  let sigint = Signal::new(SIGINT).flatten_stream();
  let sigterm = Signal::new(SIGTERM).flatten_stream();

  let stream = sigint.select(sigterm);

  {
    let prev_mask = unsafe {
      // Dirty little trick to set permissions on the socket
      libc::umask(0o177)
    };
    let socket = UnixListener::bind(&socket_path).ok_or_exit("Listen on unix socket");
    unsafe {
      libc::umask(prev_mask);
    }

    let done = socket.incoming().for_each(|socket| {
      let (reader, writer) = socket.split();

      let network = twoparty::VatNetwork::new(
        reader,
        std::io::BufWriter::new(writer),
        rpc_twoparty_capnp::Side::Server,
        Default::default(),
      );

      let rpc_system = RpcSystem::new(Box::new(network), Some(handler_factory()));
      current_thread::spawn(rpc_system.map_err(|e| error!("{:?}", e)));
      Ok(())
    });
    let autolocker = Interval::new_interval(Duration::from_secs(1)).for_each(|_| {
      check_autolock();
      Ok(())
    });

    if current_thread::block_on_all(stream.into_future().select2(done).select2(autolocker)).is_err() {
      error!("Server loop failed");
    }
  }

  info!("Cleaning up");
  if let Err(error) = fs::remove_file(&socket_path) {
    error!("Cleanup of {} failed: {}", socket_path.to_string_lossy(), error)
  }
}
