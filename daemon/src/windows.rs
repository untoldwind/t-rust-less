use std::sync::Arc;

use anyhow::Result;
use log::{error, info};
use t_rust_less_lib::service::config::LocalConfigProvider;
use t_rust_less_lib::service::local::LocalTrustlessService;
use t_rust_less_lib::service::windows::DAEMON_PIPE_NAME;
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::signal;

use crate::processor::Processor;

pub async fn run_server(service: Arc<LocalTrustlessService<LocalConfigProvider>>) -> Result<()> {
  let mut server = ServerOptions::new()
    .first_pipe_instance(true)
    .reject_remote_clients(true)
    .create(DAEMON_PIPE_NAME)?;

  info!("Listening on socket {}", DAEMON_PIPE_NAME);

  tokio::spawn(async move {
    while server.connect().await.is_ok() {
      let mut processor = Processor::new(service.clone());
      let mut client = server;

      tokio::spawn(async move {
        info!("New client connection");

        loop {
          if let Err(err) = client.readable().await {
            error!("{}", err);
            break;
          }
          let command = match processor.read_command(&mut client).await {
            Ok(Some(command)) => command,
            Ok(None) => break,
            Err(err) => {
              error!("{}", err);
              break;
            }
          };
          if let Err(err) = client.writable().await {
            error!("{}", err);
            break;
          }
          if let Err(err) = processor.process_command(&mut client, command).await {
            error!("{}", err);
            break;
          }
        }

        info!("Client disconnect");
      });

      server = match ServerOptions::new()
        .reject_remote_clients(true)
        .create(DAEMON_PIPE_NAME)
      {
        Ok(server) => server,
        Err(err) => {
          error!("{}", err);
          break;
        }
      }
    }
  });

  signal::ctrl_c().await.ok();

  Ok(())
}
