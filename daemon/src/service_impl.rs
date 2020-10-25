use crate::clipboard_control_impl::ClipboardControlImpl;
use crate::event_handler_impl::{EventHandlerClient, EventSubscriptionImpl};
use crate::secrets_store_impl::SecretsStoreImpl;
use capnp::capability::Promise;
use std::sync::Arc;
use t_rust_less_lib::api_capnp::service;
use t_rust_less_lib::service::local::LocalTrustlessService;
use t_rust_less_lib::{
  api::PasswordGeneratorParam,
  service::{StoreConfig, TrustlessService},
};

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
    let mut result = results.get().init_store_configs(store_names.len() as u32);

    for (idx, store_config) in store_names.into_iter().enumerate() {
      stry!(store_config.to_builder(result.reborrow().get(idx as u32)));
    }

    Promise::ok(())
  }

  fn upsert_store_config(
    &mut self,
    params: service::UpsertStoreConfigParams,
    _: service::UpsertStoreConfigResults,
  ) -> Promise<(), capnp::Error> {
    let store_config = stry!(params
      .get()
      .and_then(service::upsert_store_config_params::Reader::get_store_config)
      .and_then(StoreConfig::from_reader));

    stry!(self.service.upsert_store_config(store_config));

    Promise::ok(())
  }

  fn delete_store_config(
    &mut self,
    params: service::DeleteStoreConfigParams,
    _: service::DeleteStoreConfigResults,
  ) -> Promise<(), capnp::Error> {
    let store_name = stry!(params
      .get()
      .and_then(service::delete_store_config_params::Reader::get_store_name));

    stry!(self.service.delete_store_config(store_name));

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
      .set_store(capnp_rpc::new_client(SecretsStoreImpl::new(store)));

    Promise::ok(())
  }

  fn secret_to_clipboard(
    &mut self,
    params: service::SecretToClipboardParams,
    mut results: service::SecretToClipboardResults,
  ) -> Promise<(), capnp::Error> {
    let store_name = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_store_name));
    let block_id = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_block_id));
    let properties = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_properties)
      .and_then(|properties| { properties.iter().collect::<capnp::Result<Vec<&str>>>() }));
    let display_name = stry!(params
      .get()
      .and_then(service::secret_to_clipboard_params::Reader::get_display_name));

    let clipboard_control = stry!(self
      .service
      .secret_to_clipboard(store_name, block_id, &properties, &display_name));

    results
      .get()
      .set_clipboard_control(capnp_rpc::new_client(ClipboardControlImpl::new(clipboard_control)));

    Promise::ok(())
  }

  fn add_event_handler(
    &mut self,
    params: service::AddEventHandlerParams,
    mut results: service::AddEventHandlerResults,
  ) -> Promise<(), capnp::Error> {
    let handler = EventHandlerClient::new(stry!(stry!(params.get()).get_handler()));
    let subscription = stry!(self.service.add_event_handler(Box::new(handler)));

    results
      .get()
      .set_subscription(capnp_rpc::new_client(EventSubscriptionImpl(subscription)));

    Promise::ok(())
  }

  fn generate_id(
    &mut self,
    _params: service::GenerateIdParams,
    mut results: service::GenerateIdResults,
  ) -> Promise<(), capnp::Error> {
    let id = stry!(self.service.generate_id());

    results.get().set_id(&id);

    Promise::ok(())
  }

  fn generate_password(
    &mut self,
    params: service::GeneratePasswordParams,
    mut results: service::GeneratePasswordResults,
  ) -> Promise<(), capnp::Error> {
    let password_generator_param = stry!(params
      .get()
      .and_then(service::generate_password_params::Reader::get_param)
      .and_then(PasswordGeneratorParam::from_reader));
    let password = stry!(self.service.generate_password(password_generator_param));

    results.get().set_password(&password);

    Promise::ok(())
  }
}
