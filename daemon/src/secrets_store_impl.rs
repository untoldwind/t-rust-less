use capnp::capability::Promise;
use std::sync::Arc;
use t_rust_less_lib::api::{Identity, SecretListFilter, SecretVersion};
use t_rust_less_lib::api_capnp::secrets_store;
use t_rust_less_lib::memguard::SecretBytes;
use t_rust_less_lib::secrets_store::SecretsStore;

pub struct SecretsStoreImpl {
  secrets_store: Arc<SecretsStore>,
}

impl SecretsStoreImpl {
  pub fn new(secrets_store: Arc<SecretsStore>) -> Self {
    SecretsStoreImpl { secrets_store }
  }
}

impl secrets_store::Server for SecretsStoreImpl {
  fn status(
    &mut self,
    _: secrets_store::StatusParams,
    mut results: secrets_store::StatusResults,
  ) -> Promise<(), capnp::Error> {
    let status = stry!(self.secrets_store.status());

    stry!(status.to_builder(results.get().init_status()));

    Promise::ok(())
  }

  fn lock(&mut self, _: secrets_store::LockParams, _: secrets_store::LockResults) -> Promise<(), capnp::Error> {
    stry!(self.secrets_store.lock());

    Promise::ok(())
  }

  fn unlock(
    &mut self,
    params: secrets_store::UnlockParams,
    _: secrets_store::UnlockResults,
  ) -> Promise<(), capnp::Error> {
    let identity_id = stry!(params
      .get()
      .and_then(secrets_store::unlock_params::Reader::get_identity_id));
    let passphrase = SecretBytes::from_secured(stry!(params
      .get()
      .and_then(secrets_store::unlock_params::Reader::get_passphrase)));

    stry!(self.secrets_store.unlock(identity_id, passphrase));

    Promise::ok(())
  }

  fn identities(
    &mut self,
    _: secrets_store::IdentitiesParams,
    mut results: secrets_store::IdentitiesResults,
  ) -> Promise<(), capnp::Error> {
    let identities = stry!(self.secrets_store.identities());
    let mut result = results.get().init_identities(identities.len() as u32);

    for (idx, identity) in identities.into_iter().enumerate() {
      identity.to_builder(result.reborrow().get(idx as u32));
    }

    Promise::ok(())
  }

  fn add_identity(
    &mut self,
    params: secrets_store::AddIdentityParams,
    _: secrets_store::AddIdentityResults,
  ) -> Promise<(), capnp::Error> {
    let identity = stry!(params
      .get()
      .and_then(secrets_store::add_identity_params::Reader::get_identity)
      .and_then(Identity::from_reader));
    let passphrase = SecretBytes::from_secured(stry!(params
      .get()
      .and_then(secrets_store::add_identity_params::Reader::get_passphrase)));

    stry!(self.secrets_store.add_identity(identity, passphrase));

    Promise::ok(())
  }

  fn change_passphrase(
    &mut self,
    params: secrets_store::ChangePassphraseParams,
    _: secrets_store::ChangePassphraseResults,
  ) -> Promise<(), capnp::Error> {
    let passphrase = SecretBytes::from_secured(stry!(params
      .get()
      .and_then(secrets_store::change_passphrase_params::Reader::get_passphrase)));

    stry!(self.secrets_store.change_passphrase(passphrase));

    Promise::ok(())
  }

  fn list(
    &mut self,
    params: secrets_store::ListParams,
    mut results: secrets_store::ListResults,
  ) -> Promise<(), capnp::Error> {
    let filter = stry!(params
      .get()
      .and_then(secrets_store::list_params::Reader::get_filter)
      .and_then(SecretListFilter::from_reader));
    let secrets_list = stry!(self.secrets_store.list(filter));

    stry!(results.get().get_list().and_then(|l| secrets_list.to_builder(l)));

    Promise::ok(())
  }

  fn add(
    &mut self,
    params: secrets_store::AddParams,
    mut results: secrets_store::AddResults,
  ) -> Promise<(), capnp::Error> {
    let version = stry!(params
      .get()
      .and_then(secrets_store::add_params::Reader::get_version)
      .and_then(SecretVersion::from_reader));

    let block_id = stry!(self.secrets_store.add(version));

    results.get().set_block_id(&block_id);

    Promise::ok(())
  }

  fn get(
    &mut self,
    params: secrets_store::GetParams,
    mut results: secrets_store::GetResults,
  ) -> Promise<(), capnp::Error> {
    let id = stry!(params.get().and_then(secrets_store::get_params::Reader::get_id));

    let secret = stry!(self.secrets_store.get(id));

    stry!(secret.to_builder(results.get().init_secret()));

    Promise::ok(())
  }

  fn get_version(
    &mut self,
    params: secrets_store::GetVersionParams,
    mut results: secrets_store::GetVersionResults,
  ) -> Promise<(), capnp::Error> {
    let block_id = stry!(params
      .get()
      .and_then(secrets_store::get_version_params::Reader::get_block_id));

    let secret_version = stry!(self.secrets_store.get_version(block_id));

    stry!(secret_version.to_builder(results.get().init_version()));

    Promise::ok(())
  }
}
