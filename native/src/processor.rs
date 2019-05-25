use crate::messages::{Command, CommandResult, Request, Response};
use byteorder::{ByteOrder, NativeEndian};
use core::borrow::Borrow;
use log::error;
use std::io::{Read, Result, Write};
use std::sync::Arc;
use t_rust_less_lib::memguard::SecretBytes;
use t_rust_less_lib::service::TrustlessService;

pub fn process<I, O>(service: Arc<TrustlessService>, mut input: I, mut output: O) -> Result<()>
where
  I: Read,
  O: Write,
{
  let mut length_buffer = [0u8; 4];
  let mut buffer: Vec<u8> = vec![];

  loop {
    input.read_exact(&mut length_buffer)?;
    let length = NativeEndian::read_u32(&length_buffer) as usize;
    buffer.resize(length, 0);
    input.read_exact(&mut buffer)?;

    let response = match serde_json::from_slice::<Request>(&buffer) {
      Ok(request) => process_request(service.borrow(), request),
      Err(error) => {
        error!("Invalid request: {}", error);
        Response {
          id: 0,
          result: CommandResult::Invalid,
        }
      }
    };
    clear_buffer(&mut buffer);
    serde_json::to_writer(&mut buffer, &response)?;
    NativeEndian::write_u32(&mut length_buffer, buffer.len() as u32);
    output.write_all(&length_buffer)?;
    output.write_all(&buffer)?;
    clear_buffer(&mut buffer);
  }
}

fn clear_buffer(buffer: &mut Vec<u8>) {
  for b in buffer.iter_mut() {
    *b = 0
  }
  buffer.clear()
}

fn process_request(service: &TrustlessService, request: Request) -> Response {
  let result = match request.command {
    Command::ListStores => service.list_stores().into(),
    Command::GetStoreConfig(store_name) => service.get_store_config(&store_name).into(),
    Command::SetStoreConfig(config) => service.set_store_config(config).into(),
    Command::GetDefaultStore => service.get_default_store().into(),
    Command::SetDefaultStore(store_name) => service.set_default_store(&store_name).into(),
    Command::SecretToClipboard {
      store_name,
      secret_id,
      properties,
      display_name,
    } => service
      .secret_to_clipboard(
        &store_name,
        &secret_id,
        &properties.iter().map(String::as_str).collect::<Vec<&str>>(),
        &display_name,
      )
      .into(),

    Command::Status { store_name } => service
      .open_store(&store_name)
      .and_then(|store| Ok(store.status()?))
      .into(),
    Command::Lock { store_name } => service
      .open_store(&store_name)
      .and_then(|store| Ok(store.lock()?))
      .into(),
    Command::Unlock {
      store_name,
      identity_id,
      mut passphrase,
    } => {
      let passphrase_in = SecretBytes::from(passphrase.as_mut());
      service
        .open_store(&store_name)
        .and_then(move |store| Ok(store.unlock(&identity_id, passphrase_in)?))
        .into()
    }
    Command::ListIdentities { store_name } => service
      .open_store(&store_name)
      .and_then(|store| Ok(store.identities()?))
      .into(),
    Command::AddIdentity {
      store_name,
      identity,
      mut passphrase,
    } => {
      let passphrase_in = SecretBytes::from(passphrase.as_mut());
      service
        .open_store(&store_name)
        .and_then(move |store| Ok(store.add_identity(identity, passphrase_in)?))
        .into()
    }
    Command::ChangePassphrase {
      store_name,
      mut passphrase,
    } => {
      let passphrase_in = SecretBytes::from(passphrase.as_mut());
      service
        .open_store(&store_name)
        .and_then(move |store| Ok(store.change_passphrase(passphrase_in)?))
        .into()
    }
    Command::ListSecrets { store_name, filter } => service
      .open_store(&store_name)
      .and_then(move |store| Ok(store.list(filter)?))
      .into(),
    Command::AddSecret { store_name, version } => service
      .open_store(&store_name)
      .and_then(move |store| Ok(store.add(version)?))
      .into(),
    _ => CommandResult::Invalid,
  };

  Response { id: request.id, result }
}
