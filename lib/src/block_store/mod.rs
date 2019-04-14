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
/// As a rule a block store is supposed to be distributed among multiple clients, each able
/// to asynchronously create additions to it.
///
pub trait BlockStore {
  /// Get/read the (private) ring block.
  ///
  /// Every store has zero or one (private) ring containing all the relevant
  /// key material to decrypt all the other blocks.
  ///
  /// This block should be protected by some sort of passphrase/key-derivation
  ///
  fn get_ring(&self) -> StoreResult<Option<Vec<u8>>>;
  /// Set/write the (private) ring block.
  ///
  /// Implementors should ensure a sort of backup in case this operation fails, since
  /// loosing the (private) ring will render the entire store useless.
  ///
  fn store_ring(&mut self, raw: &[u8]) -> StoreResult<()>;

  /// Get/read the public ring block.
  ///
  /// The public ring contains all the public key material that does not require
  /// special protection
  ///
  fn get_public_ring(&self) -> StoreResult<Option<Vec<u8>>>;
  /// Set/write the public ring block.
  ///
  fn store_public_ring(&mut self, raw: &[u8]) -> StoreResult<()>;

  /// Get all the change logs of the store.
  ///
  /// The store has to keep track of all commits (see below).
  ///
  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>>;

  /// Get the index block of a specific client.
  ///
  /// An index block contains any sort of searchable index data referencing the
  /// underlying data blocks. As the index might contain sensible data this block
  /// has to be protected similar to a regular data block.
  ///
  /// Index blocks should not be shared among clients. I.e. every client has its own set
  /// of index blocks.
  ///
  fn get_index(&self, node: &str) -> StoreResult<Option<Vec<u8>>>;
  /// Store the index block of a specific client.
  ///
  fn store_index(&mut self, node: &str, raw: &[u8]) -> StoreResult<()>;

  /// Add a new data block to the store.
  ///
  /// Data blocks contain the secret data shared between clients and should be
  /// protected by the keys inside the ring block.
  ///
  /// The result of an add operation is a unique key of the data block.
  ///
  fn add_block(&mut self, raw: &[u8]) -> StoreResult<String>;
  /// Get a block by its id.
  ///
  fn get_block(&self, block: &str) -> StoreResult<Vec<u8>>;

  /// Commit a set of changes to the store.
  ///
  /// After adding one or more blocks to the store every client has to
  /// commit its changes. This will create an entry in the `change_log` so that
  /// other clients will notice the new data blocks.
  ///
  fn commit(&mut self, node: &str, changes: &[Change]) -> StoreResult<()>;
}

pub fn open_block_store(url: &str) -> StoreResult<Box<BlockStore>> {
  let store_url = Url::parse(url)?;

  match store_url.scheme() {
    "file" => Ok(Box::new(local_dir::LocalDirBlockStore::new(store_url.path())?)),
    "memory" => Ok(Box::new(memory::MemoryBlockStore::new())),
    _ => Err(StoreError::InvalidStoreUrl(url.to_string())),
  }
}
