use std::error::Error;
use std::io;
use std::sync::Arc;
use t_rust_less_lib::api::{Command, CommandResult};
use t_rust_less_lib::memguard::ZeroizeBytesBuffer;
use t_rust_less_lib::service::local::LocalTrustlessService;
use t_rust_less_lib::service::{ClipboardControl, ServiceError, ServiceResult, TrustlessService};
use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use zeroize::Zeroizing;

#[derive(Clone)]
pub struct Processor {
  service: Arc<LocalTrustlessService>,
  current_clipboard: Option<Arc<dyn ClipboardControl>>,
}

impl Processor {
  pub fn new(service: Arc<LocalTrustlessService>) -> Self {
    Processor {
      service,
      current_clipboard: None,
    }
  }

  pub async fn handle_connection<R, W>(&mut self, rd: &mut R, wr: &mut W) -> Result<(), Box<dyn Error>>
  where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
  {
    loop {
      let buf_len = match rd.read_u32_le().await {
        Ok(len) => len as usize,
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
        Err(err) => return Err(err.into()),
      };
      let mut buf = Zeroizing::from(vec![0; buf_len]);

      rd.read_exact(&mut buf).await?;

      self
        .process_command(wr, rmp_serde::from_read_ref(buf.as_slice())?)
        .await?;
    }
  }

  async fn process_command<W>(&mut self, wr: &mut W, command: Command) -> Result<(), Box<dyn Error>>
  where
    W: AsyncWrite + Unpin,
  {
    match command {
      Command::ListStores => write_result(wr, self.service.list_stores()).await?,
      Command::UpsertStoreConfig(config) => write_result(wr, self.service.upsert_store_config(config)).await?,
      Command::DeleteStoreConfig(name) => write_result(wr, self.service.delete_store_config(&name)).await?,
      Command::GetDefaultStore => write_result(wr, self.service.get_default_store()).await?,
      Command::SetDefaultStore(name) => write_result(wr, self.service.set_default_store(&name)).await?,
      Command::GenerateId => write_result(wr, self.service.generate_id()).await?,
      Command::GeneratePassword(param) => write_result(wr, self.service.generate_password(param)).await?,
      Command::PollEvents(last_id) => write_result(wr, self.service.poll_events(last_id)).await?,
      Command::Status(store_name) => {
        write_result(
          wr,
          self.service.open_store(&store_name).and_then(|store| store.status()),
        )
        .await?
      }
      Command::Lock(store_name) => {
        write_result(wr, self.service.open_store(&store_name).and_then(|store| store.lock())).await?
      }
      Command::Unlock {
        store_name,
        identity_id,
        passphrase,
      } => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.unlock(&identity_id, passphrase)),
        )
        .await?
      }
      Command::Identities(store_name) => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.identities()),
        )
        .await?
      }
      Command::AddIdentity {
        store_name,
        identity,
        passphrase,
      } => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.add_identity(identity, passphrase)),
        )
        .await?
      }
      Command::ChangePassphrase { store_name, passphrase } => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.change_passphrase(passphrase)),
        )
        .await?
      }
      Command::UpdateIndex(store_name) => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.update_index()),
        )
        .await?
      }
      Command::List { store_name, filter } => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.list(&filter)),
        )
        .await?
      }
      Command::Add {
        store_name,
        secret_version,
      } => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.add(secret_version)),
        )
        .await?
      }
      Command::Get { store_name, secret_id } => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.get(&secret_id)),
        )
        .await?
      }
      Command::GetVersion { store_name, block_id } => {
        write_result(
          wr,
          self
            .service
            .open_store(&store_name)
            .and_then(|store| store.get_version(&block_id)),
        )
        .await?
      }
      Command::SecretToClipboard {
        store_name,
        block_id,
        properties,
        display_name,
      } => {
        write_result(
          wr,
          match self.service.secret_to_clipboard(
            &store_name,
            &block_id,
            &properties.iter().map(String::as_str).collect::<Vec<&str>>(),
            &display_name,
          ) {
            Ok(clipboard) => {
              self.current_clipboard.replace(clipboard);
              Ok(())
            }
            Err(err) => Err(err),
          },
        )
        .await?
      }
      Command::ClipboardIsDone => match &self.current_clipboard {
        Some(clipboard) => write_result(wr, clipboard.is_done()).await?,
        None => write_result::<ServiceResult<bool>, _>(wr, Err(ServiceError::ClipboardClosed)).await?,
      },
      Command::ClipboardCurrentlyProviding => match &self.current_clipboard {
        Some(clipboard) => write_result(wr, clipboard.currently_providing()).await?,
        None => write_result::<ServiceResult<()>, _>(wr, Err(ServiceError::ClipboardClosed)).await?,
      },
      Command::ClipboardProvideNext => match &self.current_clipboard {
        Some(clipboard) => write_result(wr, clipboard.provide_next()).await?,
        None => write_result::<ServiceResult<()>, _>(wr, Err(ServiceError::ClipboardClosed)).await?,
      },
      Command::ClipboardDestroy => match &self.current_clipboard.take() {
        Some(clipboard) => write_result(wr, clipboard.destroy()).await?,
        None => write_result::<ServiceResult<()>, _>(wr, Err(ServiceError::ClipboardClosed)).await?,
      },
    }

    Ok(())
  }
}

async fn write_result<R, W>(wr: &mut W, result: R) -> Result<(), Box<dyn Error>>
where
  R: Into<CommandResult>,
  W: AsyncWrite + Unpin,
{
  let mut buf = ZeroizeBytesBuffer::with_capacity(1024);
  rmp_serde::encode::write(&mut buf, &result.into())?;

  wr.write_u32_le(buf.len() as u32).await?;
  wr.write_all(&buf).await?;

  Ok(())
}
