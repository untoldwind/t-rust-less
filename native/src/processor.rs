use crate::messages::{Command, CommandResult, Request, Response};
use byteorder::{ByteOrder, NativeEndian};
use log::error;
use std::io::{Read, Result, Write};
use std::sync::Arc;
use t_rust_less_lib::memguard::SecretBytes;
use t_rust_less_lib::secrets_store::SecretsStore;
use t_rust_less_lib::service::{ClipboardControl, ServiceResult, TrustlessService};

pub struct Processor<I, O> {
  service: Arc<dyn TrustlessService>,
  input: I,
  output: O,
  current_store: Option<(String, Arc<dyn SecretsStore>)>,
  current_clipboard: Option<Arc<dyn ClipboardControl>>,
}

impl<I, O> Processor<I, O>
where
  I: Read,
  O: Write,
{
  pub fn new(service: Arc<dyn TrustlessService>, input: I, output: O) -> Processor<I, O> {
    Processor {
      service,
      input,
      output,
      current_store: None,
      current_clipboard: None,
    }
  }
  pub fn process(&mut self) -> Result<()> {
    let mut length_buffer = [0u8; 4];
    let mut buffer: Vec<u8> = vec![];

    loop {
      self.input.read_exact(&mut length_buffer)?;
      let length = NativeEndian::read_u32(&length_buffer) as usize;
      buffer.resize(length, 0);
      self.input.read_exact(&mut buffer)?;

      let response = match serde_json::from_slice::<Request>(&buffer) {
        Ok(request) => self.process_request(request),
        Err(error) => {
          error!("Invalid request: {}", error);
          Response {
            id: 0,
            result: CommandResult::Invalid,
          }
        }
      };
      Self::clear_buffer(&mut buffer);
      serde_json::to_writer(&mut buffer, &response)?;
      NativeEndian::write_u32(&mut length_buffer, buffer.len() as u32);
      self.output.write_all(&length_buffer)?;
      self.output.write_all(&buffer)?;
      self.output.flush()?;
      Self::clear_buffer(&mut buffer);
    }
  }

  fn clear_buffer(buffer: &mut Vec<u8>) {
    for b in buffer.iter_mut() {
      *b = 0
    }
    buffer.clear()
  }

  fn process_request(&mut self, request: Request) -> Response {
    let result = match request.command {
      Command::ListStores => self.service.list_stores().into(),
      Command::GetStoreConfig(store_name) => self.service.get_store_config(&store_name).into(),
      Command::SetStoreConfig(config) => self.service.set_store_config(config).into(),
      Command::GetDefaultStore => self.service.get_default_store().into(),
      Command::SetDefaultStore(store_name) => self.service.set_default_store(&store_name).into(),
      Command::SecretToClipboard {
        store_name,
        secret_id,
        properties,
        display_name,
      } => match self.service.secret_to_clipboard(
        &store_name,
        &secret_id,
        &properties.iter().map(String::as_str).collect::<Vec<&str>>(),
        &display_name,
      ) {
        Ok(clipboard) => {
          self.current_clipboard.replace(clipboard);
          CommandResult::Success
        }
        Err(error) => {
          let display = format!("{}", error);
          CommandResult::Error { error, display }
        }
      },

      Command::Status { store_name } => self
        .open_store(&store_name)
        .and_then(|store| Ok(store.status()?))
        .into(),
      Command::Lock { store_name } => self.open_store(&store_name).and_then(|store| Ok(store.lock()?)).into(),
      Command::Unlock {
        store_name,
        identity_id,
        mut passphrase,
      } => {
        let passphrase_in = SecretBytes::from(passphrase.as_mut());
        self
          .open_store(&store_name)
          .and_then(move |store| Ok(store.unlock(&identity_id, passphrase_in)?))
          .into()
      }
      Command::ListIdentities { store_name } => self
        .open_store(&store_name)
        .and_then(|store| Ok(store.identities()?))
        .into(),
      Command::AddIdentity {
        store_name,
        identity,
        mut passphrase,
      } => {
        let passphrase_in = SecretBytes::from(passphrase.as_mut());
        self
          .service
          .open_store(&store_name)
          .and_then(move |store| Ok(store.add_identity(identity, passphrase_in)?))
          .into()
      }
      Command::ChangePassphrase {
        store_name,
        mut passphrase,
      } => {
        let passphrase_in = SecretBytes::from(passphrase.as_mut());
        self
          .service
          .open_store(&store_name)
          .and_then(move |store| Ok(store.change_passphrase(passphrase_in)?))
          .into()
      }
      Command::ListSecrets { store_name, filter } => self
        .open_store(&store_name)
        .and_then(move |store| Ok(store.list(filter)?))
        .into(),
      Command::GetSecret { store_name, secret_id } => self
        .open_store(&store_name)
        .and_then(move |store| Ok(store.get(&secret_id)?))
        .into(),
      Command::AddSecret { store_name, version } => self
        .open_store(&store_name)
        .and_then(move |store| Ok(store.add(version)?))
        .into(),
      Command::GetSecretVersion { store_name, block_id } => self
        .open_store(&store_name)
        .and_then(move |store| Ok(store.get_version(&block_id)?))
        .into(),

      Command::ClipboardIsDone => match &self.current_clipboard {
        Some(clipboard) => clipboard.is_done().into(),
        None => CommandResult::Bool(true),
      },
      Command::ClipboardCurrentlyProviding => match &self.current_clipboard {
        Some(clipboard) => clipboard.currently_providing().into(),
        None => CommandResult::Empty,
      },
      Command::ClipboardDestroy => match &self.current_clipboard {
        Some(clipboard) => clipboard.destroy().into(),
        None => CommandResult::Success,
      },
      _ => CommandResult::Invalid,
    };

    Response { id: request.id, result }
  }

  fn open_store(&mut self, store_name: &str) -> ServiceResult<Arc<dyn SecretsStore>> {
    match &self.current_store {
      Some((name, store)) if name == store_name => Ok(store.clone()),
      _ => match self.service.open_store(store_name) {
        Ok(store) => {
          self.current_store.replace((store_name.to_string(), store.clone()));
          Ok(store)
        }
        err => err,
      },
    }
  }
}
