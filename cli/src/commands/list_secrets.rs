use crate::commands::tui::create_tui;
use crate::commands::unlock_store;
use crate::error::ExtResult;
use atty::Stream;
use cursive::event::Key;
use cursive::theme::Effect;
use cursive::traits::{Boxable, Identifiable, Scrollable};
use cursive::utils::markup::StyledString;
use cursive::views::{DummyView, EditView, LinearLayout, SelectView};
use cursive::Cursive;
use std::sync::Arc;
use t_rust_less_lib::api::{SecretEntry, SecretEntryMatch, SecretListFilter};
use t_rust_less_lib::secrets_store::SecretsStore;
use t_rust_less_lib::service::TrustlessService;

pub fn list_secrets(service: Arc<TrustlessService>, store_name: String, filter: SecretListFilter) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));

  let status = secrets_store.status().ok_or_exit("Get status");

  if status.locked {
    unlock_store(&secrets_store, &store_name);
  }

  if atty::is(Stream::Stdout) {
    list_secrets_ui(secrets_store, filter)
  } else {
    let list = secrets_store.list(filter).ok_or_exit("List entries");

    for entry in list.entries {
      println!("{:?}", entry);
    }
  }
}

struct ListUIState {
  secrets_store: Arc<SecretsStore>,
  filter: SecretListFilter,
}

fn list_secrets_ui(secrets_store: Arc<SecretsStore>, initial_filter: SecretListFilter) {
  let mut list = secrets_store.list(initial_filter.clone()).ok_or_exit("List entries");
  list.entries.sort();

  let mut name_search = EditView::new();
  if let Some(name_filter) = &initial_filter.name {
    name_search.set_content(name_filter.to_string());
  }
  name_search.set_on_edit(update_name_filter);
  let mut entry_select = SelectView::new();
  entry_select.add_all(list.entries.into_iter().map(entry_list_item));

  let state = ListUIState {
    secrets_store,
    filter: initial_filter,
  };

  let mut siv = create_tui();

  siv.set_user_data(state);
  siv.add_global_callback(Key::Esc, Cursive::quit);
  siv.add_fullscreen_layer(
    LinearLayout::vertical()
      .child(name_search.with_id("name_search").full_width())
      .child(
        LinearLayout::horizontal()
          .child(entry_select.with_id("entry_list").full_width().scrollable())
          .child(DummyView {}.full_screen()),
      ),
  );

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

  let mut entry_select = s.find_id::<SelectView<SecretEntry>>("entry_list").unwrap();
  entry_select.clear();
  entry_select.add_all(next_entries.into_iter().map(entry_list_item));
}

fn entry_list_item(entry_match: SecretEntryMatch) -> (StyledString, SecretEntry) {
  let name = &entry_match.entry.name;
  let mut styled_name = StyledString::new();
  let mut last = 0usize;

  for highlight in entry_match.name_highlights {
    if highlight > last {
      styled_name.append_plain(&name[last..highlight]);
    }
    styled_name.append_styled(&name[highlight..=highlight], Effect::Reverse);
    last = highlight + 1;
  }
  styled_name.append_plain(&name[last..]);

  (styled_name, entry_match.entry)
}
