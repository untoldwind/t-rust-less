use crate::processor::Processor;
use futures::future;
use log::{error, info};
use std::error::Error;
use std::fs;
use std::sync::Arc;
use t_rust_less_lib::service::local::LocalTrustlessService;
use t_rust_less_lib::service::unix::daemon_socket_path;
use tokio::net::UnixListener;
use tokio::signal;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn run_server(service: Arc<LocalTrustlessService>) -> Result<(), Box<dyn Error>> {
  let socket_path = daemon_socket_path();

  info!("Listening on socket {}", socket_path.to_string_lossy());

  let prev_mask = unsafe {
    // Dirty little trick to set permissions on the socket
    libc::umask(0o177)
  };
  let listener = UnixListener::bind(&socket_path)?;
  unsafe { libc::umask(prev_mask) };

  tokio::spawn(async move {
    while let Ok((mut socket, _)) = listener.accept().await {
      let mut processor = Processor::new(service.clone());

      tokio::spawn(async move {
        let (mut rd, mut wr) = socket.split();

        info!("New client connection");

        if let Err(err) = handle_connection(&mut processor, &mut rd, &mut wr).await {
          error!("{}", err);
        }

        info!("Client disconnect");
      });
    }
  });

  future::select(
    Box::pin(async {
      signal::ctrl_c().await.ok();
    }),
    Box::pin(async {
      if let Ok(mut signal) = signal::unix::signal(signal::unix::SignalKind::terminate()) {
        signal.recv().await;
      }
    }),
  )
  .await;

  info!("Cleaning up");
  if let Err(error) = fs::remove_file(&socket_path) {
    error!("Cleanup of {} failed: {}", socket_path.to_string_lossy(), error)
  }

  Ok(())
}

async fn handle_connection<R, W>(processor: &mut Processor, rd: &mut R, wr: &mut W) -> Result<(), Box<dyn Error>>
where
  R: AsyncRead + Unpin,
  W: AsyncWrite + Unpin,
{
  loop {
    let command = match processor.read_command(rd).await? {
      Some(command) => command,
      None => return Ok(()),
    };

    processor.process_command(wr, command).await?;
  }
}
