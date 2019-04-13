use std::process;

use atty::Stream;
use cursive::traits::{Boxable, Identifiable};
use cursive::views::{Dialog, EditView, LinearLayout, TextView};

use crate::commands::tui::create_tui;
use crate::config::Config;
use cursive::event::Key;

pub fn init(maybe_config: Option<Config>) {
  if !atty::is(Stream::Stdout) {
    println!("Please use a terminal");
    process::exit(1);
  }

  let mut siv = create_tui();

  siv.add_global_callback(Key::Esc, |s| s.quit());

  siv.add_layer(
    Dialog::around(
      LinearLayout::vertical()
        .child(TextView::new("t-rust-less configuration"))
        .child(EditView::new().with_id("store_dir").fixed_width(60)),
    )
    .title("t-rust-less configuration")
    .padding_left(5)
    .padding_right(5)
    .padding_top(1)
    .padding_bottom(1)
    .button("Quit2", |s| s.quit())
    .button("Quit", |s| s.quit()),
  );

  siv.run();
}
