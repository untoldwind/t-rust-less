use t_rust_less_lib::clipboard::SelectionProvider;

mod cli;

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
