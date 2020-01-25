mod add_identity;
mod export;
mod import;
mod init;
mod list_identities;
mod list_secrets;
mod lock;
mod status;
pub mod tui;
mod unlock;

pub use self::add_identity::*;
pub use self::export::*;
pub use self::import::*;
pub use self::init::*;
pub use self::list_identities::*;
pub use self::list_secrets::*;
pub use self::lock::*;
pub use self::status::*;
pub use self::unlock::*;

use rand::{distributions, thread_rng, Rng};

fn generate_id(length: usize) -> String {
  let rng = thread_rng();

  rng
    .sample_iter(&distributions::Alphanumeric)
    .take(length)
    .collect::<String>()
}
