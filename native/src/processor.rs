use crate::input::Input;
use crate::messages::{Command, CommandResult, Request, Response};
use crate::output::Output;
use log::error;
use std::io::{Read, Result, Write};
use std::sync::Arc;
use t_rust_less_lib::memguard::SecretBytes;
use t_rust_less_lib::secrets_store::{SecretStoreResult, SecretsStore};
use t_rust_less_lib::service::{ClipboardControl, TrustlessService};

pub struct Processor<I, O> {
  service: Arc<dyn TrustlessService>,
  input: Input<I>,
  output: Arc<Output<O>>,
  current_store: Option<(String, Arc<dyn SecretsStore>)>,
  current_clipboard: Option<Arc<dyn ClipboardControl>>,
}

impl<I, O> Processor<I, O>
where
  I: Read,
  O: Write + 'static + Sync + Send,
{
  pub fn new(service: Arc<dyn TrustlessService>, input: I, raw_output: O) -> Result<Processor<I, O>> {
    let output = Arc::new(Output::new(raw_output));

    Ok(Processor {
      service,
      input: Input::new(input),
      output,
      current_store: None,
      current_clipboard: None,
    })
  }

  pub fn process(&mut self) -> Result<()> {
    loop {
      let response = match self.input.read::<Request>()? {
        Some(request) => self.process_request(request),
        None => {
          error!("Invalid request");
          Response::Command {
            id: 0,
            result: CommandResult::Invalid,
          }
        }
      };
      self.input.clear_buffer();
      self.output.send(&response)?;
    }
  }

  fn process_request(&mut self, request: Request) -> Response {
    let result = match request.command {
      Command::ListStores => self.service.list_stores().into(),
      Command::DeleteStoreConfig(store_name) => self.service.delete_store_config(&store_name).into(),
      Command::UpsertStoreConfig(config) => self.service.upsert_store_config(config).into(),
      Command::GetDefaultStore => self.service.get_default_store().into(),
      Command::SetDefaultStore(store_name) => self.service.set_default_store(&store_name).into(),
      Command::SecretToClipboard {
        store_name,
        block_id,
        properties,
      } => match self.service.secret_to_clipboard(
        &store_name,
        &block_id,
        &properties.iter().map(String::as_str).collect::<Vec<&str>>(),
      ) {
        Ok(clipboard) => {
          self.current_clipboard.replace(clipboard);
          CommandResult::Success
        }
        Err(error) => {
          let display = format!("{error}");
          CommandResult::Error { error, display }
        }
      },

      Command::Status { store_name } => self.open_store(&store_name).and_then(|store| store.status()).into(),
      Command::Lock { store_name } => self.open_store(&store_name).and_then(|store| store.lock()).into(),
      Command::Unlock {
        store_name,
        identity_id,
        passphrase,
      } => {
        let passphrase_in = SecretBytes::from(passphrase);
        self
          .open_store(&store_name)
          .and_then(move |store| store.unlock(&identity_id, passphrase_in))
          .into()
      }
      Command::ListIdentities { store_name } => {
        self.open_store(&store_name).and_then(|store| store.identities()).into()
      }
      Command::AddIdentity {
        store_name,
        identity,
        passphrase,
      } => {
        let passphrase_in = SecretBytes::from(passphrase);
        self
          .service
          .open_store(&store_name)
          .and_then(move |store| store.add_identity(identity, passphrase_in))
          .into()
      }
      Command::ChangePassphrase { store_name, passphrase } => {
        let passphrase_in = SecretBytes::from(passphrase);
        self
          .service
          .open_store(&store_name)
          .and_then(move |store| store.change_passphrase(passphrase_in))
          .into()
      }
      Command::ListSecrets { store_name, filter } => self
        .open_store(&store_name)
        .and_then(move |store| store.list(&filter))
        .into(),
      Command::GetSecret { store_name, secret_id } => self
        .open_store(&store_name)
        .and_then(move |store| store.get(&secret_id))
        .into(),
      Command::AddSecret { store_name, version } => self
        .open_store(&store_name)
        .and_then(move |store| store.add(version))
        .into(),
      Command::GetSecretVersion { store_name, block_id } => self
        .open_store(&store_name)
        .and_then(move |store| store.get_version(&block_id))
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

    Response::Command { id: request.id, result }
  }

  fn open_store(&mut self, store_name: &str) -> SecretStoreResult<Arc<dyn SecretsStore>> {
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
