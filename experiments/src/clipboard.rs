use log::info;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use t_rust_less_lib::clipboard::{Clipboard, SelectionProvider};

struct DummyProvider {
  counter: u32,
}

impl SelectionProvider for DummyProvider {
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

pub fn experimental_clipboard() {
  let clipboard = Arc::new(Clipboard::new(DummyProvider { counter: 0 }).unwrap());

  thread::spawn({
    let cloned = clipboard.clone();
    move || {
      thread::sleep(Duration::from_secs(10));
      info!("Destroy clipboard");
      cloned.destroy();
    }
  });

  clipboard.wait().unwrap();
}
