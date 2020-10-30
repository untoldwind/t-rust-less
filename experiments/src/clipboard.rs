use log::info;
use std::env;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use t_rust_less_lib::api::{Event, EventHub};
use t_rust_less_lib::clipboard::{Clipboard, SelectionProvider};

struct DummyProvider {
  counter: u32,
}

impl SelectionProvider for DummyProvider {
  fn current_selection_name(&self) -> Option<String> {
    if self.counter < 10 {
      Some("counter".to_string())
    } else {
      None
    }
  }

  fn get_selection(&mut self) -> Option<String> {
    self.counter += 1;

    if self.counter < 10 {
      info!("Providing {}", self.counter);
      Some(format!("Something {}", self.counter))
    } else {
      None
    }
  }
}

struct TestEventHub;

impl EventHub for TestEventHub {
  fn send(&self, _event: Event) {}
}

pub fn experimental_clipboard() {
  let clipboard = Arc::new(
    Clipboard::new(
      &env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()),
      DummyProvider { counter: 0 },
      "Store".to_string(),
      "SecretId".to_string(),
      Arc::new(TestEventHub),
    )
    .unwrap(),
  );

  thread::spawn({
    let cloned = clipboard.clone();
    move || {
      thread::sleep(Duration::from_secs(30));
      info!("Destroy clipboard");
      cloned.destroy();
    }
  });

  clipboard.wait().unwrap();
}
