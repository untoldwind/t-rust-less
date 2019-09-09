use capnp::capability::Promise;
use futures::Future;
use log::error;
use t_rust_less_lib::api::{Event, EventHandler, EventSubscription};
use t_rust_less_lib::api_capnp::{event_handler, event_subscription};
use tokio::runtime::current_thread;

pub struct EventHandlerClient {
  client: event_handler::Client,
}

impl EventHandlerClient {
  pub fn new(client: event_handler::Client) -> EventHandlerClient {
    EventHandlerClient { client }
  }
}

impl EventHandler for EventHandlerClient {
  fn handle(&self, event: Event) {
    let mut request = self.client.handle_request();

    current_thread::spawn(
      match event.to_builder(request.get().init_event()) {
        Ok(_) => request.send().promise,
        Err(err) => Promise::err(err),
      }
      .and_then(|response| {
        response.get()?;

        Ok(())
      })
      .map_err(|err| {
        error!("Failed handing error: {}", err);
      }),
    )
  }
}

pub struct EventSubscriptionImpl(pub Box<dyn EventSubscription>);

impl event_subscription::Server for EventSubscriptionImpl {}
