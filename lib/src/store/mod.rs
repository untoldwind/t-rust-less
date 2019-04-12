
pub use self::model::*;
use url::Url;

mod error;
mod local_dir;
mod memory;
mod model;

#[cfg(test)]
mod tests;

pub use self::error::{StoreError, StoreResult};

/// Common interface of all block stores
///
/// In terms of persistence t-rust-less thinks in collections of blocks. Whereas a
/// block is just a chunk of bytes (i.e. Vec<u8>) stored on some sort of medium that
/// may or may not be available to the public (!!!).
///
/// To put it even more bluntly: All data inside a block has to be protected in a way
/// that the underlying medium might as well be twitter or facebook (not that I actually
/// suggest or plan to write an implementation of that sort). The blcok store in itself
/// is not responsible providing this sort of protection ... is just stores and organizes
/// blocks.
///
pub trait Store {
  /// Get/read the (private) ring block.
  ///
  /// Every store has zero or one (private) ring containing all the relevant
  /// key material to decrypt all the other blocks.
  ///
  /// This block should be protected by some sort of passphrase/key-derivation
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
