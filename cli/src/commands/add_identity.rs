use cursive::views::Dialog;
use cursive::Cursive;
use std::sync::Arc;
use t_rust_less_lib::secrets_store::SecretsStore;

pub fn add_identity_dialog(siv: &mut Cursive, secrets_store: Arc<SecretsStore>, title: &str) {
  siv.set_user_data(secrets_store);
  siv.add_layer(Dialog::new().title(title))
}
