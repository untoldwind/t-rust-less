use log::info;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use t_rust_less_lib::clipboard::{Clipboard, SelectionProvider};
use std::time::SystemTime;

struct DummyProvider {
  counter: u32,
  last_request: Option<SystemTime>,
}

impl SelectionProvider for DummyProvider {
  fn get_selection(&mut self) -> Option<String> {
    let now = SystemTime::now();
    if self.last_request.is_none() || now.duration_since(self.last_request.unwrap()).unwrap().as_millis() > 200 {
      self.counter += 1;
      self.last_request = Some(now);
    }

    if self.counter < 10 {
      info!("Providing {}", self.counter);
      Some(format!("Something {}", self.counter))
    } else {
      None
    }
  }
}

pub fn experimental_clipboard() {
  let clipboard = Arc::new(Clipboard::new(DummyProvider { counter: 0, last_request: None }).unwrap());

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
