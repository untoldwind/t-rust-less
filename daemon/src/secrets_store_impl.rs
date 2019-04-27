use capnp::capability::Promise;
use std::sync::Arc;
use t_rust_less_lib::api::Identity;
use t_rust_less_lib::secrets_store::SecretsStore;
use t_rust_less_lib::service_capnp::{identity, secrets_store};

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
    let mut result_status = stry!(results.get().get_status());

    result_status.set_locked(status.locked);
    match status.unlocked_by {
      Some(identity) => build_identity(stry!(result_status.reborrow().get_unlocked_by()).init_some(), identity),
      None => stry!(result_status.reborrow().get_unlocked_by()).set_none(()),
    }
    match status.autolock_at {
      Some(autolock_at) => result_status.set_autolock_at(autolock_at.timestamp_millis()),
      None => result_status.set_autolock_at(std::i64::MIN),
    }
    result_status.set_version(&status.version);

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
    _: secrets_store::IdentitiesResults,
  ) -> Promise<(), ::capnp::Error> {
    Promise::err(::capnp::Error::unimplemented("method not implemented".to_string()))
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

fn build_identity(mut builder: identity::Builder, identity: Identity) {
  builder.set_id(&identity.id);
  builder.set_name(&identity.name);
  builder.set_email(&identity.email);
}
