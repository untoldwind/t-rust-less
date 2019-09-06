use capnp::capability::Promise;
use std::sync::Arc;
use t_rust_less_lib::api_capnp::clipboard_control;
use t_rust_less_lib::service::ClipboardControl;

pub struct ClipboardControlImpl {
  clipboard_control: Arc<dyn ClipboardControl>,
}

impl ClipboardControlImpl {
  pub fn new(clipboard_control: Arc<dyn ClipboardControl>) -> Self {
    ClipboardControlImpl { clipboard_control }
  }
}

impl clipboard_control::Server for ClipboardControlImpl {
  fn is_done(
    &mut self,
    _: clipboard_control::IsDoneParams,
    mut results: clipboard_control::IsDoneResults,
  ) -> Promise<(), capnp::Error> {
    let result = stry!(self.clipboard_control.is_done());

    results.get().set_is_done(result);

    Promise::ok(())
  }

  fn currently_providing(
    &mut self,
    _: clipboard_control::CurrentlyProvidingParams,
    mut results: clipboard_control::CurrentlyProvidingResults,
  ) -> Promise<(), capnp::Error> {
    let mut result = results.get().init_providing();
    match stry!(self.clipboard_control.currently_providing()) {
      Some(providing) => {
        let text = stry!(capnp::text::new_reader(providing.as_bytes()));
        stry!(result.set_some(text))
      }
      None => result.set_none(()),
    }

    Promise::ok(())
  }

  fn destroy(
    &mut self,
    _: clipboard_control::DestroyParams,
    _: clipboard_control::DestroyResults,
  ) -> Promise<(), capnp::Error> {
    stry!(self.clipboard_control.destroy());

    Promise::ok(())
  }
}
