use crate::commands::tui::create_tui;
use crate::commands::unlock_store;
use crate::error::ExtResult;
use crate::view::{SecretView, StatusView};
use atty::Stream;
use chrono::{DateTime, Utc};
use cursive::event::{Event, Key};
use cursive::theme::Effect;
use cursive::traits::{Boxable, Identifiable, Scrollable};
use cursive::utils::markup::StyledString;
use cursive::views::{EditView, LinearLayout, SelectView, TextContent};
use cursive::Cursive;
use std::env;
use std::sync::Arc;
use t_rust_less_lib::api::{
  SecretEntry, SecretEntryMatch, SecretListFilter, Status, PROPERTY_PASSWORD, PROPERTY_TOTP_URL, PROPERTY_USERNAME,
};
use t_rust_less_lib::secrets_store::SecretsStore;
use t_rust_less_lib::service::TrustlessService;

pub fn list_secrets(service: Arc<dyn TrustlessService>, store_name: String, filter: SecretListFilter) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));

  let mut status = secrets_store.status().ok_or_exit("Get status");

  if atty::is(Stream::Stdout) {
    let mut siv = create_tui();
    if status.locked {
      status = unlock_store(&mut siv, &secrets_store, &store_name);
    }

    let initial_state = ListUIState {
      service,
      store_name,
      secrets_store,
      filter,
      status_text: TextContent::new(status_text(&status)),
      last_update: None,
    };
    list_secrets_ui(&mut siv, initial_state, status);
  } else {
    let list = secrets_store.list(filter).ok_or_exit("List entries");

    for entry in list.entries {
      println!("{:?}", entry);
    }
  }
}

struct ListUIState {
  service: Arc<dyn TrustlessService>,
  store_name: String,
  secrets_store: Arc<dyn SecretsStore>,
  filter: SecretListFilter,
  status_text: TextContent,
  last_update: Option<DateTime<Utc>>,
}

fn list_secrets_ui(siv: &mut Cursive, initial_state: ListUIState, status: Status) {
  let list = initial_state
    .secrets_store
    .list(initial_state.filter.clone())
    .ok_or_exit("List entries");

  let mut name_search = EditView::new();
  if let Some(name_filter) = &initial_state.filter.name {
    name_search.set_content(name_filter.to_string());
  }
  name_search.set_on_edit(update_name_filter);
  let mut entry_select = SelectView::new();
  let initial_selected = list.entries.first().map(|e| e.entry.id.clone());
  entry_select.add_all(list.entries.into_iter().map(entry_list_item));
  entry_select.set_on_select(update_selection);

  let service = initial_state.service.clone();
  let store_name = initial_state.store_name.clone();
  let secrets_store = initial_state.secrets_store.clone();

  siv.set_fps(2);
  siv.add_global_callback(Key::Esc, Cursive::quit);
  siv.add_global_callback(
    Event::CtrlChar('a'),
    secret_to_clipboard(&[PROPERTY_USERNAME, PROPERTY_PASSWORD, PROPERTY_TOTP_URL]),
  );
  siv.add_global_callback(Event::CtrlChar('u'), secret_to_clipboard(&[PROPERTY_USERNAME]));
  siv.add_global_callback(Event::CtrlChar('p'), secret_to_clipboard(&[PROPERTY_PASSWORD]));
  siv.add_global_callback(Event::CtrlChar('o'), secret_to_clipboard(&[PROPERTY_TOTP_URL]));
  siv.add_global_callback(Event::Refresh, update_status);
  siv.add_fullscreen_layer(
    LinearLayout::vertical()
      .child(
        LinearLayout::horizontal()
          .child(name_search.with_name("name_search").full_width())
          .child(
            StatusView::new(secrets_store.clone(), status)
              .with_name("status")
              .fixed_width(14),
          ),
      )
      .child(
        LinearLayout::horizontal()
          .child(entry_select.with_name("entry_list").full_width().scrollable())
          .child(
            SecretView::new(service, store_name, secrets_store, initial_selected)
              .with_name("secret_view")
              .full_screen(),
          ),
      ),
  );
  siv.set_user_data(initial_state);

  siv.run()
}

fn update_name_filter(s: &mut Cursive, name_filter: &str, _: usize) {
  let next_entries = {
    let state = s.user_data::<ListUIState>().unwrap();
    state.filter.name = if name_filter.is_empty() {
      None
    } else {
      Some(name_filter.to_string())
    };

    let mut list = state
      .secrets_store
      .list(state.filter.clone())
      .ok_or_exit("List entries");
    list.entries.sort();
    list.entries
  };

  let mut entry_select = s.find_name::<SelectView<SecretEntry>>("entry_list").unwrap();
  let mut secret_view = s.find_name::<SecretView>("secret_view").unwrap();
  match next_entries.first() {
    Some(new_selection) => secret_view.show_secret(&new_selection.entry.id),
    None => secret_view.clear(),
  }
  entry_select.clear();
  entry_select.add_all(next_entries.into_iter().map(entry_list_item));
}

fn update_selection(s: &mut Cursive, entry: &SecretEntry) {
  let mut secret_view = s.find_name::<SecretView>("secret_view").unwrap();
  secret_view.show_secret(&entry.id);
}

fn entry_list_item(entry_match: SecretEntryMatch) -> (StyledString, SecretEntry) {
  let name = &entry_match.entry.name;
  let mut styled_name = StyledString::new();
  let mut last = 0usize;

  for highlight in entry_match.name_highlights {
    if highlight > last {
      styled_name.append_plain(name.chars().skip(last).take(highlight - last).collect::<String>());
    }
    styled_name.append_styled(
      name.chars().skip(highlight).take(1).collect::<String>(),
      Effect::Reverse,
    );
    last = highlight + 1;
  }
  styled_name.append_plain(name.chars().skip(last).collect::<String>());

  (styled_name, entry_match.entry)
}

fn secret_to_clipboard(properties: &'static [&'static str]) -> impl Fn(&mut Cursive) -> () {
  move |s: &mut Cursive| {
    let maybe_entry = {
      let entry_select = s.find_name::<SelectView<SecretEntry>>("entry_list").unwrap();
      entry_select.selection()
    };
    let state = s.user_data::<ListUIState>().unwrap();

    if let Some(entry) = maybe_entry {
      state
        .service
        .secret_to_clipboard(
          &state.store_name,
          &entry.id,
          properties,
          &env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string()),
        )
        .ok_or_exit("Copy to clipboard");
    }
  }
}

fn update_status(s: &mut Cursive) {
  let next_status = {
    let state = s.user_data::<ListUIState>().unwrap();
    let now = Utc::now();
    if state.last_update.is_none() || (now - state.last_update.unwrap()).num_milliseconds() > 400 {
      state.last_update.replace(now);
      match state.secrets_store.status() {
        Ok(status) => {
          state.status_text.set_content(status_text(&status));
          Some(status)
        }
        _ => None,
      }
    } else {
      None
    }
  };
  if next_status.is_some() && next_status.unwrap().locked {
    s.quit()
  }
}

fn status_text(status: &Status) -> String {
  if status.locked {
    " Locked".to_string()
  } else {
    match status.autolock_at {
      Some(autolock_at) => {
        let timeout = autolock_at - Utc::now();

        format!(" Unlocked {}s", timeout.num_seconds())
      }
      None => " Unlocked".to_string(),
    }
  }
}
