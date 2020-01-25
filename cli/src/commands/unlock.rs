use crate::commands::tui::create_tui;
use crate::error::ExtResult;
use crate::view::PasswordView;
use atty::Stream;
use cursive::event::Key;
use cursive::traits::{Boxable, Identifiable};
use cursive::views::{Dialog, DummyView, LinearLayout, SelectView, TextView};
use cursive::Cursive;
use std::process;
use std::sync::Arc;
use t_rust_less_lib::api::{Identity, Status};
use t_rust_less_lib::secrets_store::SecretsStore;
use t_rust_less_lib::service::TrustlessService;

pub fn unlock(service: Arc<dyn TrustlessService>, store_name: String) {
  let secrets_store = service
    .open_store(&store_name)
    .ok_or_exit(format!("Failed opening store {}: ", store_name));

  let status = secrets_store.status().ok_or_exit("Get status");

  if status.locked {
    let mut siv = create_tui();

    unlock_store(&mut siv, &secrets_store, &store_name);
  }
}

pub fn unlock_store(siv: &mut Cursive, secrets_store: &Arc<dyn SecretsStore>, name: &str) -> Status {
  if !atty::is(Stream::Stdout) {
    println!("Please use a terminal");
    process::exit(1);
  }

  let identities = secrets_store.identities().ok_or_exit("Get identities");

  if identities.is_empty() {
    println!("Store does not have any identities to unlock");
    process::exit(1)
  }

  unlock_dialog(siv, secrets_store, name, identities);

  let status = secrets_store.status().ok_or_exit("Get status");

  if status.locked {
    println!("Unlock failed");
    process::exit(1);
  }

  status
}

fn unlock_dialog(siv: &mut Cursive, secrets_store: &Arc<dyn SecretsStore>, name: &str, identities: Vec<Identity>) {
  siv.set_user_data(secrets_store.clone());
  siv.add_global_callback(Key::Esc, Cursive::quit);
  siv.add_layer(
    Dialog::around(
      LinearLayout::vertical()
        .child(TextView::new("Identity"))
        .child(
          SelectView::new()
            .with_all(
              identities
                .into_iter()
                .map(|i| (format!("{} <{}>", i.name, i.email), i.id)),
            )
            .with_name("identity")
            .fixed_width(50),
        )
        .child(DummyView {})
        .child(TextView::new("Passphrase"))
        .child(
          PasswordView::new(100)
            .on_submit(do_unlock_store)
            .with_name("passphrase"),
        ),
    )
    .title(format!("Unlock store {}", name))
    .button("Unlock", do_unlock_store)
    .button("Abort", Cursive::quit)
    .padding_left(5)
    .padding_right(5)
    .padding_top(1)
    .padding_bottom(1),
  );

  siv.focus_name("passphrase").unwrap();

  siv.run();
}

fn do_unlock_store(s: &mut Cursive) {
  let secrets_store = s.user_data::<Arc<dyn SecretsStore>>().unwrap().clone();
  let maybe_identity = s.find_name::<SelectView>("identity").unwrap().selection();
  let passphrase = s.find_name::<PasswordView>("passphrase").unwrap().get_content();
  let identity_id = match maybe_identity {
    Some(id) => id,
    _ => {
      s.add_layer(Dialog::info("No identity selected"));
      return;
    }
  };

  if let Err(error) = secrets_store.unlock(&identity_id, passphrase) {
    s.add_layer(Dialog::info(format!("Unable to unlock store:\n{}", error)));
    return;
  }

  s.quit()
}
