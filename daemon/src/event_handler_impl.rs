use futures::executor::LocalSpawner;
use futures::task::LocalSpawn;
use futures::FutureExt;
use log::error;
use t_rust_less_lib::api::{Event, EventHandler, EventSubscription};
use t_rust_less_lib::api_capnp::{event_handler, event_subscription};

pub struct EventHandlerClient {
  client: event_handler::Client,
  spawner: LocalSpawner,
}

impl EventHandlerClient {
  pub fn new(client: event_handler::Client, spawner: LocalSpawner) -> EventHandlerClient {
    EventHandlerClient { client, spawner }
  }
}

impl EventHandler for EventHandlerClient {
  fn handle(&self, event: Event) {
    let mut request = self.client.handle_request();

    if let Err(err) = event.to_builder(request.get().init_event()) {
      error!("Failed encoding event: {}", err);
      return;
    }

    if let Err(err) = self.spawner.spawn_local_obj(
      Box::pin(request.send().promise.map(|r| {
        if let Err(err) = r {
          error!("Event receiver error: {}", err);
        }
      }))
      .into(),
    ) {
      error!("Failed sending event: {}", err);
      return;
    }
  }
}

pub struct EventSubscriptionImpl(pub Box<dyn EventSubscription>);

impl event_subscription::Server for EventSubscriptionImpl {}
