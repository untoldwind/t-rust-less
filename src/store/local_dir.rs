use std::sync::Mutex;
use std::io::prelude::*;
use std::fs::{File, OpenOptions};
use std::path::PathBuf;
use super::{Store, Change, ChangeLog};
use super::error::{ErrorKind, Error, Result};

pub struct LocalDir {
    lock: Mutex<()>,
    base_dir: PathBuf,
}

impl LocalDir {
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> LocalDir {
        LocalDir {
            lock: Mutex::new(()),
            base_dir: base_dir.into(),
        }
    }
}

impl Store for LocalDir {
    fn get_ring(&self) -> Result<Vec<u8>> {
        let _guard = self.lock.lock()?;
        let mut ring_file = OpenOptions::new().read(true).open(self.base_dir.join("ring"))?;
        let mut content = vec![];

        ring_file.read_to_end(&mut content)?;

        Ok(content)
    }

    fn store_ring(&self, raw : &[u8]) -> Result<()>{
        let maybe_current = match self.get_ring() {
            Ok(content) => Some(content),
            Err(Error(ErrorKind::Io(ref io_err), _)) if io_err.kind() == ::std::io::ErrorKind::NotFound => None,
            Err(err) => bail!(err),
        };

        let _guard = self.lock.lock()?;

        match maybe_current {
            Some(current) => {
                let mut backup_file = OpenOptions::new().write(true).create(true).truncate(true).open(self.base_dir.join("ring.bak"))?;

                backup_file.write(&current)?;
                backup_file.flush()?;
                backup_file.sync_all()?;
            },
            _ => (),
        }

        let mut ring_file = OpenOptions::new().write(true).create(true).truncate(true).open(self.base_dir.join("ring"))?;

        ring_file.write(raw)?;
        ring_file.flush()?;
        ring_file.sync_all()?;
        Ok(())
    }
    
    fn get_public_ring(&self) -> Result<Vec<u8>>{
        let _guard = self.lock.lock()?;
        let mut ring_file = OpenOptions::new().read(true).open(self.base_dir.join("ring.pub"))?;
        let mut content = vec![];

        ring_file.read_to_end(&mut content)?;

        Ok(content)
    }

    fn store_public_ring(&self, raw: &[u8]) -> Result<()>{
        let maybe_current = match self.get_ring() {
            Ok(content) => Some(content),
            Err(Error(ErrorKind::Io(ref io_err), _)) if io_err.kind() == ::std::io::ErrorKind::NotFound => None,
            Err(err) => bail!(err),
        };

        let _guard = self.lock.lock()?;

        match maybe_current {
            Some(current) => {
                let mut backup_file = OpenOptions::new().write(true).create(true).truncate(true).open(self.base_dir.join("ring.pub,bak"))?;

                backup_file.write(&current)?;
                backup_file.flush()?;
                backup_file.sync_all()?;
            },
            _ => (),
        }

        let mut ring_file = OpenOptions::new().write(true).create(true).truncate(true).open(self.base_dir.join("ring.pub"))?;

        ring_file.write(raw)?;
        ring_file.flush()?;
        ring_file.sync_all()?;
        Ok(())
    }
    
    fn change_logs(&self) -> Result<Vec<ChangeLog>>{
        unimplemented!()
    }
    
    fn get_index(&self, node: &String) -> Result<Vec<u8>>{
        unimplemented!()
    }
    fn store_index(&self, node: &String, raw: &[u8]) -> Result<()>{
        unimplemented!()
    }
    
    fn add_block(&self, raw: &[u8]) -> Result<String>{
        unimplemented!()
    }
    fn get_block(&self, block: &String) -> Result<Vec<u8>>{
        unimplemented!()
    }

    fn commit(&self, node: &String, changes: &[Change]) -> Result<()>{
        unimplemented!()
    }

}