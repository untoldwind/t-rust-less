use log::info;
use std::env;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use t_rust_less_lib::api::{ClipboardProviding, EventData, EventHub};
use t_rust_less_lib::clipboard::{Clipboard, ClipboardCommon, SelectionProvider};
use zeroize::Zeroizing;

#[derive(Clone)]
struct DummyProvider {
  counter: u32,
}

impl SelectionProvider for DummyProvider {
  fn current_selection(&self) -> Option<ClipboardProviding> {
    if self.counter < 10 {
      Some(ClipboardProviding {
        store_name: "Store".to_string(),
        block_id: "Block".to_string(),
        secret_name: "Secret".to_string(),
        property: "Property".to_string(),
      })
    } else {
      None
    }
  }

  fn get_selection_value(&self) -> Option<Zeroizing<String>> {
    if self.counter < 10 {
      info!("Providing {}", self.counter);
      Some(Zeroizing::new(format!("Something {}", self.counter)))
    } else {
      None
    }
  }

  fn next_selection(&mut self) {
    self.counter += 1;
  }
}

struct TestEventHub;

impl EventHub for TestEventHub {
  fn send(&self, _event: EventData) {}
}

pub fn experimental_clipboard() {
  let clipboard = Arc::new(
    Clipboard::new(
      &env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()),
      DummyProvider { counter: 0 },
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

  loop {
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).unwrap();

    clipboard.provide_next();

    if buffer.trim() == "c" {
      break;
    }
  }
  clipboard.destroy();
  //  clipboard.wait().unwrap();
}
