use std::{pin::Pin, sync::Arc};

use chrono::Utc;
use futures::Future;
use log::debug;
use t_rust_less_lib::service::TrustlessService;
use tokio::{spawn, time::sleep, time::Duration};

pub fn start_sync_loop(service: Arc<dyn TrustlessService>) {
  spawn(trigger_sync(service));
}

fn trigger_sync(service: Arc<dyn TrustlessService>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
  Box::pin(async move {
    let millis = match service.synchronize() {
      Some(next_run) => (next_run - Utc::now()).num_milliseconds(),
      _ => 0,
    };
    debug!("Trigger sync: Next sync in {millis} millis");

    let duration = Duration::from_millis(if millis > 0 { millis as u64 } else { 1000 });
    spawn(async move {
      sleep(duration).await;

      trigger_sync(service).await;
    });
  })
}
