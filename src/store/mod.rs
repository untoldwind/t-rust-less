mod error;
mod model;
mod local_dir;

use self::error::Result;
pub use self::model::*;

trait Store {
    fn get_ring(&self) -> Result<Option<Vec<u8>>>;
    fn store_ring(&self, raw: &[u8]) -> Result<()>;

    fn get_public_ring(&self) -> Result<Option<Vec<u8>>>;
    fn store_public_ring(&self, raw: &[u8]) -> Result<()>;

    fn change_logs(&self) -> Result<Vec<ChangeLog>>;

    fn get_index(&self, node: &String) -> Result<Option<Vec<u8>>>;
    fn store_index(&self, node: &String, raw: &[u8]) -> Result<()>;

    fn add_block(&self, raw: &[u8]) -> Result<String>;
    fn get_block(&self, block: &String) -> Result<Vec<u8>>;

    fn commit(&self, node: &String, changes: &[Change]) -> Result<()>;
}
