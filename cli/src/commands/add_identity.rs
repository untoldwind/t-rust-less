use crate::commands::generate_id;
use crate::commands::tui::create_tui;
use crate::view::PasswordView;
use anyhow::{bail, Context, Result};
use atty::Stream;
use clap::Args;
use cursive::event::Key;
use cursive::traits::{Nameable, Resizable};
use cursive::views::{Dialog, DummyView, EditView, LinearLayout, TextView};
use cursive::Cursive;
use std::sync::Arc;
use t_rust_less_lib::api::Identity;
use t_rust_less_lib::secrets_store::SecretsStore;
use t_rust_less_lib::service::TrustlessService;

#[derive(Debug, Args)]
pub struct AddIdentitiesCommand {}

impl AddIdentitiesCommand {
  pub fn run(self, service: Arc<dyn TrustlessService>, store_name: String) -> Result<()> {
    if !atty::is(Stream::Stdout) {
      bail!("Please use a terminal");
    }

    let secrets_store = service
      .open_store(&store_name)
      .with_context(|| format!("Failed opening store {}: ", store_name))?;
    let mut siv = create_tui();

    siv.add_global_callback(Key::Esc, Cursive::quit);

    add_identity_dialog(&mut siv, secrets_store, "Add identity");

    siv.run();

    Ok(())
  }
}

pub fn add_identity_dialog(siv: &mut Cursive, secrets_store: Arc<dyn SecretsStore>, title: &str) {
  siv.set_user_data(secrets_store);
  siv.add_layer(
    Dialog::around(
      LinearLayout::vertical()
        .child(TextView::new("Id"))
        .child(EditView::new().content(generate_id(40)).disabled().with_name("id"))
        .child(DummyView {})
        .child(TextView::new("Name"))
        .child(EditView::new().with_name("name").fixed_width(50))
        .child(DummyView {})
        .child(TextView::new("Email"))
        .child(EditView::new().with_name("email").fixed_width(50))
        .child(DummyView {})
        .child(TextView::new("Passphrase"))
        .child(PasswordView::new(100).with_name("passphrase")),
    )
    .title(title)
    .button("Create", create_identity)
    .button("Abort", Cursive::quit)
    .padding_left(5)
    .padding_right(5)
    .padding_top(1)
    .padding_bottom(1),
  )
}

fn create_identity(s: &mut Cursive) {
  let identity = Identity {
    id: s.find_name::<EditView>("id").unwrap().get_content().to_string(),
    name: s.find_name::<EditView>("name").unwrap().get_content().to_string(),
    email: s.find_name::<EditView>("email").unwrap().get_content().to_string(),
    hidden: false,
  };
  let passphrase = s.find_name::<PasswordView>("passphrase").unwrap().get_content();

  if identity.id.is_empty() {
    s.add_layer(Dialog::info("Id must not be empty"));
    return;
  }
  if identity.name.is_empty() {
    s.add_layer(Dialog::info("Name must not be empty"));
    return;
  }
  if identity.email.is_empty() {
    s.add_layer(Dialog::info("Email must not be empty"));
    return;
  }

  let secrets_store: &Arc<dyn SecretsStore> = s.user_data().unwrap();
  match secrets_store.add_identity(identity, passphrase) {
    Ok(_) => s.quit(),
    Err(error) => s.add_layer(Dialog::info(format!("Failed to create identity: {}", error))),
  }
}
