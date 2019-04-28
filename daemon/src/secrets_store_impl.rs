use capnp::capability::Promise;
use std::sync::Arc;
use t_rust_less_lib::api_capnp::secrets_store;
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
  ) -> Promise<(), ::capnp::Error> {
    let status = stry!(self.secrets_store.status());
    let result_status = stry!(results.get().get_status());

    stry!(status.to_builder(result_status));

    Promise::ok(())
  }

  fn lock(&mut self, _: secrets_store::LockParams, _: secrets_store::LockResults) -> Promise<(), ::capnp::Error> {
    Promise::err(::capnp::Error::unimplemented("method not implemented".to_string()))
  }

  fn unlock(&mut self, _: secrets_store::UnlockParams, _: secrets_store::UnlockResults) -> Promise<(), ::capnp::Error> {
    Promise::err(::capnp::Error::unimplemented("method not implemented".to_string()))
  }

  fn identities(
    &mut self,
    _: secrets_store::IdentitiesParams,
    mut results: secrets_store::IdentitiesResults,
  ) -> Promise<(), ::capnp::Error> {
    let identities = stry!(self.secrets_store.identities());
    let mut result = results.get().init_identities(identities.len() as u32);

    for (idx, identity) in identities.into_iter().enumerate() {
      identity.to_builder(result.reborrow().get(idx as u32));
    }

    Promise::ok(())
  }

  fn add_identity(
    &mut self,
    _: secrets_store::AddIdentityParams,
    _: secrets_store::AddIdentityResults,
  ) -> Promise<(), ::capnp::Error> {
    Promise::err(::capnp::Error::unimplemented("method not implemented".to_string()))
  }

  fn change_passphrase(
    &mut self,
    _: secrets_store::ChangePassphraseParams,
    _: secrets_store::ChangePassphraseResults,
  ) -> Promise<(), ::capnp::Error> {
    Promise::err(::capnp::Error::unimplemented("method not implemented".to_string()))
  }
}
