use std::process;

use atty::Stream;
use cursive::traits::{Boxable, Identifiable};
use cursive::views::{Dialog, DummyView, EditView, LinearLayout, TextView};
use cursive::Cursive;

use crate::commands::add_identity::add_identity_dialog;
use crate::commands::generate_id;
use crate::commands::tui::create_tui;
use crate::config::{default_autolock_timeout, default_store_dir};
use crate::error::exit_with_error;
use cursive::event::Key;
use std::fs;
use std::sync::Arc;
use t_rust_less_lib::service::{StoreConfig, TrustlessService};
use url::Url;

pub fn init(service: Arc<dyn TrustlessService>, maybe_store_name: Option<String>) {
  if !atty::is(Stream::Stdout) {
    println!("Please use a terminal");
    process::exit(1);
  }

  let store_name = maybe_store_name.unwrap_or_else(|| "t-rust-less-store".to_string());
  let store_configs = match service.list_stores() {
    Ok(configs) => configs,
    Err(err) => {
      exit_with_error(
        format!("Checking exsting configuration for store {}: ", store_name),
        err,
      );
      unreachable!()
    }
  };
  let maybe_config = store_configs
    .iter()
    .find(|config| config.name.as_str() == store_name.as_str());
  let store_path = match maybe_config {
    Some(ref config) => match Url::parse(&config.store_url) {
      Ok(url) => url.path().to_string(),
      _ => default_store_dir(&store_name).to_string_lossy().to_string(),
    },
    _ => default_store_dir(&store_name).to_string_lossy().to_string(),
  };
  let autolock_timeout_secs = match maybe_config {
    Some(ref config) => config.autolock_timeout_secs,
    _ => default_autolock_timeout().as_secs(),
  };

  let mut siv = create_tui();

  siv.set_user_data(service);
  siv.add_global_callback(Key::Esc, Cursive::quit);

  siv.add_layer(
    Dialog::around(
      LinearLayout::vertical()
        .child(TextView::new("Store name"))
        .child(EditView::new().content(store_name).disabled().with_name("store_name"))
        .child(DummyView {})
        .child(TextView::new("Store directory"))
        .child(
          EditView::new()
            .content(collapse_path(store_path))
            .with_name("store_dir")
            .fixed_width(60),
        )
        .child(DummyView {})
        .child(TextView::new("Auto-lock timeout (sec)"))
        .child(
          EditView::new()
            .content(autolock_timeout_secs.to_string())
            .with_name("autolock_timeout"),
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
  let service = s.user_data::<Arc<dyn TrustlessService>>().unwrap().clone();
  let store_name = s.find_name::<EditView>("store_name").unwrap().get_content();
  let store_path = expand_path(&s.find_name::<EditView>("store_dir").unwrap().get_content());
  let autolock_timeout = s.find_name::<EditView>("autolock_timeout").unwrap().get_content();
  let autolock_timeout_secs = try_with_dialog!(
    autolock_timeout.parse::<u64>(),
    s,
    "Autolock timeout has to be a positive integer:\n{}"
  );
  let store_configs = try_with_dialog!(service.list_stores(), s, "Failed reading existing configuration:\n{}");
  let client_id = match store_configs
    .iter()
    .find(|config| config.name.as_str() == store_name.as_str())
  {
    Some(previous) => previous.client_id.clone(),
    None => generate_id(64),
  };

  if store_path.is_empty() {
    s.add_layer(Dialog::info("Store directory must not be empty"));
    return;
  }
  try_with_dialog!(fs::create_dir_all(&store_path), s, "Failed creating directory:\n{}");

  let store_url = Url::from_directory_path(store_path).unwrap();
  let secrets_store_url = format!("multilane+{}", store_url.to_string());
  let config = StoreConfig {
    name: store_name.to_string(),
    client_id,
    store_url: secrets_store_url,
    autolock_timeout_secs,
    default_identity_id: None,
  };

  try_with_dialog!(service.upsert_store_config(config), s, "Failed to store config:\n{}");

  let secrets_store = try_with_dialog!(
    service.open_store(&store_name),
    s,
    "Unable to open store {}:\n{}",
    store_name
  );
  let identities = try_with_dialog!(secrets_store.identities(), s, "Unable to query identities:\n{}");

  if identities.is_empty() {
    s.pop_layer();

    add_identity_dialog(s, secrets_store, "Create initial identity");
    return;
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
