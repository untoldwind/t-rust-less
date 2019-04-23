use std::process;

use atty::Stream;
use cursive::traits::{Boxable, Identifiable};
use cursive::views::{Dialog, DummyView, EditView, LinearLayout, TextView};
use cursive::Cursive;

use crate::commands::add_identity::add_identity_dialog;
use crate::commands::tui::create_tui;
use crate::config::{default_autolock_timeout, default_store_dir, write_config, Config};
use cursive::event::Key;
use rand::{distributions, thread_rng, Rng};
use std::fs;
use std::time::Duration;
use t_rust_less_lib::secrets_store::open_secrets_store;
use url::Url;

pub fn init(maybe_config: Option<Config>) {
  if !atty::is(Stream::Stdout) {
    println!("Please use a terminal");
    process::exit(1);
  }

  let store_path = match maybe_config {
    Some(ref config) => match Url::parse(&config.store_url) {
      Ok(url) => url.path().to_string(),
      _ => default_store_dir().to_string_lossy().to_string(),
    },
    _ => default_store_dir().to_string_lossy().to_string(),
  };
  let autolock_timeout_secs = match maybe_config {
    Some(ref config) => config.autolock_timeout.as_secs(),
    _ => default_autolock_timeout().as_secs(),
  };

  let mut siv = create_tui();

  if let Some(config) = maybe_config {
    siv.set_user_data(config)
  }

  siv.add_global_callback(Key::Esc, Cursive::quit);

  siv.add_layer(
    Dialog::around(
      LinearLayout::vertical()
        .child(TextView::new("Store directory"))
        .child(
          EditView::new()
            .content(collapse_path(store_path))
            .with_id("store_dir")
            .fixed_width(60),
        )
        .child(DummyView {})
        .child(TextView::new("Auto-lock timeout (sec)"))
        .child(
          EditView::new()
            .content(autolock_timeout_secs.to_string())
            .with_id("autolock_timeout"),
        ),
    )
    .button("Abort", Cursive::quit)
    .button("Store", store_config)
    .title("t-rust-less configuration")
    .padding_left(5)
    .padding_right(5)
    .padding_top(1)
    .padding_bottom(1),
  );

  siv.run();
}

macro_rules! try_with_dialog {
  ($result:expr, $siv:expr, $format:expr $(, $args:expr )*) => {
    match $result {
      Ok(result) => result,
      Err(error) => {
        $siv.add_layer(Dialog::info(format!($format $(, $args )*, error)));
        return;
      }
    }
  };
}

fn store_config(s: &mut Cursive) {
  let store_path = expand_path(&s.find_id::<EditView>("store_dir").unwrap().get_content());
  let autolock_timeout_secs = s.find_id::<EditView>("store_dir").unwrap().get_content();
  let autolock_timeout = Duration::from_secs(try_with_dialog!(
    autolock_timeout_secs.parse::<u64>(),
    s,
    "Autolock timeout has to be a positive integer:\n{}"
  ));
  let client_id = match s.user_data::<Config>() {
    Some(previous) => previous.client_id.clone(),
    _ => generate_client_id(),
  };

  if store_path.is_empty() {
    s.add_layer(Dialog::info("Store directory must not be empty"));
    return;
  }
  try_with_dialog!(fs::create_dir_all(&store_path), s, "Failed creating directory:\n{}");

  let store_url = Url::from_directory_path(store_path).unwrap();
  let secrets_store_url = format!("multilane+{}", store_url.to_string());

  let secrets_store = try_with_dialog!(
    open_secrets_store(&secrets_store_url, &client_id, autolock_timeout),
    s,
    "Unable to open store {}:\n{}",
    secrets_store_url
  );
  let identities = try_with_dialog!(secrets_store.identities(), s, "Unable to query identities:\n{}");

  let config = Config {
    client_id,
    store_url: secrets_store_url,
    autolock_timeout,
  };

  try_with_dialog!(write_config(&config), s, "Failed to store config:\n{}");

  if identities.is_empty() {
    s.pop_layer();

    add_identity_dialog(s, secrets_store, "Create initial identity");
  }

  s.quit();
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

fn expand_path(path: &str) -> String {
  match dirs::home_dir() {
    Some(home_dir) => path.replace("~", &home_dir.to_string_lossy()),
    None => path.to_string(),
  }
}

fn generate_client_id() -> String {
  let mut rng = thread_rng();

  rng
    .sample_iter(&distributions::Alphanumeric)
    .take(64)
    .collect::<String>()
}
