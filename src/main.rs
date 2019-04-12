use crate::clipboard::SelectionProvider;

mod api;
mod cli;
mod secret_store;
#[allow(dead_code)]
mod secret_store_capnp {
  include!(concat!(env!("OUT_DIR"), "/src/secret_store/secret_store_capnp.rs"));
}
mod clipboard;
mod memguard;
mod store;

pub struct TestSel {
  counter: u32,
}

impl SelectionProvider for TestSel {
  fn get_selection(&mut self) -> Option<String> {
    self.counter += 1;

    if self.counter < 10 {
      Some(format!("blabla {}\n", self.counter))
    } else {
      None
    }
  }
}

fn main() {
  /*
  let clip = Clipboard::new(TestSel { counter: 0}).unwrap();

  dbg!("Waiting for order");
  clip.wait().unwrap();
  */

  cli::cli_run()
}
