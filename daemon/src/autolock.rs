use std::{sync::Arc, time::Duration};

use t_rust_less_lib::service::TrustlessService;
use tokio::time::interval;

pub fn start_autolock_loop(service: Arc<dyn TrustlessService>) {
  let mut interval = interval(Duration::from_secs(1));
  tokio::spawn(async move {
    loop {
      interval.tick().await;
      service.check_autolock();
    }
  });
}
