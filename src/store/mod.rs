pub use self::model::*;
use url::Url;

mod error;
mod model;
mod local_dir;

pub use self::error::{StoreResult, StoreError};

pub trait Store {
    fn get_ring(&self) -> StoreResult<Option<Vec<u8>>>;
    fn store_ring(&self, raw: &[u8]) -> StoreResult<()>;

    fn get_public_ring(&self) -> StoreResult<Option<Vec<u8>>>;
    fn store_public_ring(&self, raw: &[u8]) -> StoreResult<()>;

    fn change_logs(&self) -> StoreResult<Vec<ChangeLog>>;

    fn get_index(&self, node: &String) -> StoreResult<Option<Vec<u8>>>;
    fn store_index(&self, node: &String, raw: &[u8]) -> StoreResult<()>;

    fn add_block(&self, raw: &[u8]) -> StoreResult<String>;
    fn get_block(&self, block: &String) -> StoreResult<Vec<u8>>;

    fn commit(&self, node: &String, changes: &[Change]) -> StoreResult<()>;
}

impl Store {
    fn new(url: &String) -> StoreResult<Box<Store>> {
        let store_url = Url::parse(url)?;

        match store_url.scheme() {
            "file" => Ok(Box::new(local_dir::LocalDir::new(store_url.path()))),
            _ => Err(StoreError::InvalidStoreUrl(url.clone())),
        }
    }
}
