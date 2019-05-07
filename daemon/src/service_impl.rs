use crate::secrets_store_impl::SecretsStoreImpl;
use capnp::capability::Promise;
use std::sync::Arc;
use t_rust_less_lib::api_capnp::{secrets_store, service};
use t_rust_less_lib::service::local::LocalTrustlessService;
use t_rust_less_lib::service::{StoreConfig, TrustlessService};

pub struct ServiceImpl {
  service: Arc<LocalTrustlessService>,
}

impl ServiceImpl {
  pub fn new(service: Arc<LocalTrustlessService>) -> Self {
    ServiceImpl { service }
  }
}

impl service::Server for ServiceImpl {
  fn list_stores(
    &mut self,
    _: service::ListStoresParams,
    mut results: service::ListStoresResults,
  ) -> Promise<(), capnp::Error> {
    let store_names = stry!(self.service.list_stores());
    let mut result = results.get().init_store_names(store_names.len() as u32);

    for (idx, store_name) in store_names.into_iter().enumerate() {
      result.set(idx as u32, &store_name);
    }

    Promise::ok(())
  }

  fn set_store_config(
    &mut self,
    params: service::SetStoreConfigParams,
    _: service::SetStoreConfigResults,
  ) -> Promise<(), capnp::Error> {
    let store_config = stry!(params
      .get()
      .and_then(service::set_store_config_params::Reader::get_store_config)
      .and_then(StoreConfig::from_reader));

    stry!(self.service.set_store_config(store_config));

    Promise::ok(())
  }

  fn get_store_config(
    &mut self,
    params: service::GetStoreConfigParams,
    mut results: service::GetStoreConfigResults,
  ) -> Promise<(), capnp::Error> {
    let store_name = stry!(params
      .get()
      .and_then(service::get_store_config_params::Reader::get_store_name));
    let store_config = stry!(self.service.get_store_config(store_name));

    store_config.to_builder(stry!(results.get().get_store_config()));

    Promise::ok(())
  }

  fn get_default_store(
    &mut self,
    _: service::GetDefaultStoreParams,
    mut results: service::GetDefaultStoreResults,
  ) -> Promise<(), capnp::Error> {
    let mut result = results.get().init_store_name();

    match stry!(self.service.get_default_store()) {
      Some(default_store) => {
        let text = stry!(capnp::text::new_reader(default_store.as_bytes()));
        stry!(result.set_some(text))
      }
      None => result.set_none(()),
    }

    Promise::ok(())
  }

  fn set_default_store(
    &mut self,
    params: service::SetDefaultStoreParams,
    _: service::SetDefaultStoreResults,
  ) -> Promise<(), capnp::Error> {
    let store_name = stry!(params
      .get()
      .and_then(service::set_default_store_params::Reader::get_store_name));

    stry!(self.service.set_default_store(store_name));

    Promise::ok(())
  }

  fn open_store(
    &mut self,
    params: service::OpenStoreParams,
    mut results: service::OpenStoreResults,
  ) -> Promise<(), capnp::Error> {
    let store_name = stry!(params
      .get()
      .and_then(service::open_store_params::Reader::get_store_name));
    let store = stry!(self.service.open_store(store_name));

    results
      .get()
      .set_store(secrets_store::ToClient::new(SecretsStoreImpl::new(store)).into_client::<capnp_rpc::Server>());

    Promise::ok(())
  }

  fn secret_to_clipboard(
    &mut self,
    params: service::SecretToClipboardParams,
    _: service::SecretToClipboardResults,
  ) -> Promise<(), capnp::Error> {
    let store_name = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_store_name));
    let secret_id = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_secret_id));
    let properties = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_properties)
      .and_then(|properties| { properties.iter().collect::<capnp::Result<Vec<&str>>>() }));
    let display_name = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_display_name));

    stry!(self
      .service
      .secret_to_clipboard(store_name, secret_id, &properties, &display_name));

    Promise::ok(())
  }
}
