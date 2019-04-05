pub use self::model::*;
use url::Url;

mod error;
mod local_dir;
mod memory;
mod model;

#[cfg(test)]
mod tests;

pub use self::error::{StoreError, StoreResult};

pub trait Store {
  fn get_ring(&self) -> StoreResult<Option<Vec<u8>>>;
  fn store_ring(&mut self, raw: &[u8]) -> StoreResult<()>;

  fn get_public_ring(&self) -> StoreResult<Option<Vec<u8>>>;
  fn store_public_ring(&mut self, raw: &[u8]) -> StoreResult<()>;

  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>>;

  fn get_index(&self, node: &str) -> StoreResult<Option<Vec<u8>>>;
  fn store_index(&mut self, node: &str, raw: &[u8]) -> StoreResult<()>;

  fn add_block(&mut self, raw: &[u8]) -> StoreResult<String>;
  fn get_block(&self, block: &str) -> StoreResult<Vec<u8>>;

  fn commit(&mut self, node: &str, changes: &[Change]) -> StoreResult<()>;
}

fn open_store(url: &str) -> StoreResult<Box<Store>> {
  let store_url = Url::parse(url)?;

  match store_url.scheme() {
    "file" => Ok(Box::new(local_dir::LocalDir::new(store_url.path()))),
      "memory" => Ok(Box::new(memory::Memory::new())),
    _ => Err(StoreError::InvalidStoreUrl(url.to_string())),
  }
}
