use std::process;

use atty::Stream;
use cursive::traits::{Boxable, Identifiable};
use cursive::views::{Dialog, EditView, LinearLayout, TextView};

use crate::commands::tui::create_tui;
use crate::config::{default_store_dir, Config};
use cursive::event::Key;
use url::Url;

pub fn init(maybe_config: Option<Config>) {
  if !atty::is(Stream::Stdout) {
    println!("Please use a terminal");
    process::exit(1);
  }

  let store_path = match maybe_config {
    Some(config) => match Url::parse(&config.store_url) {
      Ok(url) => url.path().to_string(),
      _ => default_store_dir().to_string_lossy().to_string(),
    },
    _ => default_store_dir().to_string_lossy().to_string(),
  };

  let mut siv = create_tui();

  siv.add_global_callback(Key::Esc, |s| s.quit());

  siv.add_layer(
    Dialog::around(
      LinearLayout::vertical()
        .child(TextView::new("t-rust-less configuration"))
        .child(
          EditView::new()
            .content(collapse_path(store_path))
            .with_id("store_dir")
            .fixed_width(60),
        ),
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

fn collapse_path(path: String) -> String {
  match dirs::home_dir() {
    Some(home_dir) => {
      let prefix: &str = &home_dir.to_string_lossy();
      path.replace(prefix, "~")
    }
    None => path,
  }
}

fn expand_path(path: String) -> String {
  match dirs::home_dir() {
    Some(home_dir) => path.replace("~", &home_dir.to_string_lossy()),
    None => path,
  }
}
