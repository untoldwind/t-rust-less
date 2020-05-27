use futures::FutureExt;
use log::error;
use t_rust_less_lib::api::{Event, EventHandler, EventSubscription};
use t_rust_less_lib::api_capnp::{event_handler, event_subscription};
use tokio::task;

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

    if let Err(err) = event.to_builder(request.get().init_event()) {
      error!("Failed encoding event: {}", err);
      return;
    }

    task::spawn_local(Box::pin(request.send().promise.map(|r| {
      if let Err(err) = r {
        error!("Event receiver error: {}", err);
      }
    })));
  }
}

pub struct EventSubscriptionImpl(pub Box<dyn EventSubscription>);

impl event_subscription::Server for EventSubscriptionImpl {}
