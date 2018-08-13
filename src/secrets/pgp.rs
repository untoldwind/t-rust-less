use std::sync::Mutex;
use store::Store;
use api::Identity;
use super::error::Result;

pub struct PGPSecrets {
    lock: Mutex<()>,
    store: Box<Store>,
    scrypted: bool,
    node_id: String,
    identities: Vec<Identity>,
    master_key_bits: u32,
}

impl PGPSecrets {
    fn new(store_url: &String, scrypted: bool, master_key_bits: u32) -> Result<PGPSecrets> {
        unimplemented!()
    }
}