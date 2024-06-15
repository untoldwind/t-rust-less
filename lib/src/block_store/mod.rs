pub use self::model::*;
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use url::Url;

#[cfg(feature = "dropbox")]
pub mod dropbox;
mod error;
mod local_dir;
mod local_wal;
mod memory;
mod model;
#[cfg(feature = "sled")]
mod sled;
pub mod sync;

#[cfg(test)]
mod tests;

pub use self::error::{StoreError, StoreResult};
use crate::memguard::weak::ZeroingWords;

type RingId = (String, u64);
type RingContent = (u64, ZeroingWords);

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
/// All implementation are supposed to be thread-safe. Any kind of internal state has to be
/// protected accordingly.
///
pub trait BlockStore: std::fmt::Debug + Send + Sync {
  /// Get the current node id.
  ///
  /// Each accessor to a distributed store should have a unique id.
  ///
  fn node_id(&self) -> &str;

  /// Get list of ring block identifiers.
  ///
  /// A store may contain any number of secrets rings. Usually associated with identities/users
  /// that may access the store.
  fn list_ring_ids(&self) -> StoreResult<Vec<RingId>>;

  /// Get/read the ring block by its id.
  ///
  /// Every identities/user should have a ring block containing all the relevant key material to
  /// encrypt/descript all the other blocks.
  ///
  /// Theses block should be protected by some sort of passphrase/key-derivation
  ///
  fn get_ring(&self, ring_id: &str) -> StoreResult<RingContent>;

  /// Set/write a ring block.
  ///
  /// Implementors should ensure a sort of backup in case this operation fails, since
  /// loosing the (private) ring will render the entire store useless to a user
  ///
  fn store_ring(&self, ring_id: &str, version: u64, raw: &[u8]) -> StoreResult<()>;

  /// Get all the change logs of the store.
  ///
  /// The store has to keep track of all commits (see below).
  ///
  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>>;

  /// Get the index block of a specific client/user.
  ///
  /// An index block contains any sort of searchable index data referencing the
  /// underlying data blocks. As the index might contain sensible data this block
  /// has to be protected similar to a regular data block.
  ///
  /// Index blocks should not be shared among clients or user. I.e. every client/user
  /// should have its own set index blocks.
  ///
  fn get_index(&self, index_id: &str) -> StoreResult<Option<ZeroingWords>>;

  /// Store the index block of a specific client/user.
  ///
  fn store_index(&self, index_id: &str, raw: &[u8]) -> StoreResult<()>;

  /// Add a new data block to the store.
  ///
  /// Data blocks contain the secret data shared between clients and should be
  /// protected by the keys inside the ring block.
  ///
  /// The result of an add operation is a unique key of the data block.
  ///
  fn add_block(&self, raw: &[u8]) -> StoreResult<String>;
  /// Get a block by its id.
  ///
  fn get_block(&self, block: &str) -> StoreResult<ZeroingWords>;

  /// Commit a set of changes to the store.
  ///
  /// After adding one or more blocks to the store every client has to
  /// commit its changes. This will create an entry in the `change_log` so that
  /// other clients will notice the new data blocks.
  ///
  fn commit(&self, changes: &[Change]) -> StoreResult<()>;

  /// Update changelog of other nodes.
  ///
  /// This is intended for store synchronization only.
  fn update_change_log(&self, change_log: ChangeLog) -> StoreResult<()>;
}

pub fn open_block_store(url: &str, node_id: &str) -> StoreResult<Arc<dyn BlockStore>> {
  let store_url = Url::parse(url)?;

  match store_url.scheme() {
    "file" => Ok(Arc::new(local_dir::LocalDirBlockStore::new(
      store_url.to_file_path().unwrap(),
      node_id,
    )?)),
    "wal" => Ok(Arc::new(local_wal::LocalWalBlockStore::new(
      store_url.to_file_path().unwrap(),
      node_id,
    )?)),
    "memory" => Ok(Arc::new(memory::MemoryBlockStore::new(node_id))),
    #[cfg(feature = "sled")]
    "sled" => Ok(Arc::new(sled::SledBlockStore::new(
      store_url.to_file_path().unwrap(),
      node_id,
    )?)),
    #[cfg(feature = "dropbox")]
    "dropbox" => Ok(Arc::new(dropbox::DropboxBlockStore::new(
      store_url.username(),
      store_url.host_str().unwrap(),
      node_id,
    )?)),
    _ => Err(StoreError::InvalidStoreUrl(url.to_string())),
  }
}

pub fn generate_block_id(data: &[u8]) -> String {
  let mut hasher = Sha256::new();

  hasher.update(data);

  HEXLOWER.encode(&hasher.finalize())
}
