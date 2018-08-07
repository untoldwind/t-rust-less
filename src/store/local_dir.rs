use std::sync::Mutex;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::fs::{read_dir, DirBuilder, File, OpenOptions};
use std::path::{Path, PathBuf};
use openssl::hash::{Hasher, MessageDigest};
use data_encoding::HEXLOWER;
use super::{Change, ChangeLog, Operation, Store};
use super::error::Result;

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
    fn get_ring(&self) -> Result<Option<Vec<u8>>> {
        let _guard = self.lock.lock()?;
        read_optional_file(self.base_dir.join("ring"))
    }

    fn store_ring(&self, raw: &[u8]) -> Result<()> {
        let maybe_current = self.get_ring()?;
        let _guard = self.lock.lock()?;

        match maybe_current {
            Some(current) => {
                let mut backup_file = File::create(self.base_dir.join("ring.bak"))?;

                backup_file.write(&current)?;
                backup_file.flush()?;
                backup_file.sync_all()?;
            }
            _ => (),
        }

        let mut ring_file = File::create(self.base_dir.join("ring"))?;

        ring_file.write(raw)?;
        ring_file.flush()?;
        ring_file.sync_all()?;
        Ok(())
    }

    fn get_public_ring(&self) -> Result<Option<Vec<u8>>> {
        let _guard = self.lock.lock()?;
        read_optional_file(self.base_dir.join("ring.pub"))
    }

    fn store_public_ring(&self, raw: &[u8]) -> Result<()> {
        let maybe_current = self.get_public_ring()?;
        let _guard = self.lock.lock()?;

        match maybe_current {
            Some(current) => {
                let mut backup_file = File::create(self.base_dir.join("ring.pub,bak"))?;

                backup_file.write(&current)?;
                backup_file.flush()?;
                backup_file.sync_all()?;
            }
            _ => (),
        }

        let mut ring_file = File::create(self.base_dir.join("ring.pub"))?;

        ring_file.write(raw)?;
        ring_file.flush()?;
        ring_file.sync_all()?;
        Ok(())
    }

    fn change_logs(&self) -> Result<Vec<ChangeLog>> {
        let _guard = self.lock.lock()?;
        let commit_dir = read_dir(self.base_dir.join("logs"))?;
        let mut change_logs: Vec<ChangeLog> = vec![];

        for maybe_entry in commit_dir {
            let entry = maybe_entry?;

            if !entry.metadata()?.is_file() {
                continue;
            }
            change_logs.push(parse_change_log(
                &entry.file_name().to_string_lossy(),
                &entry.path(),
            )?);
        }

        Ok(change_logs)
    }

    fn get_index(&self, node: &String) -> Result<Option<Vec<u8>>> {
        let _guard = self.lock.lock()?;
        read_optional_file(self.base_dir.join("indexes").join(node))
    }

    fn store_index(&self, node: &String, raw: &[u8]) -> Result<()> {
        DirBuilder::new()
            .recursive(true)
            .create(self.base_dir.join("indexes"))?;
        let mut index_file = File::create(self.base_dir.join("indexes").join(node))?;

        index_file.write(raw)?;
        index_file.flush()?;

        Ok(())
    }

    fn add_block(&self, raw: &[u8]) -> Result<String> {
        DirBuilder::new()
            .recursive(true)
            .create(self.base_dir.join("blocks"))?;
        let block_id = generate_id(raw)?;
        let mut block_file = File::create(self.base_dir.join("blocks").join(&block_id))?;

        block_file.write(raw)?;
        block_file.flush()?;

        Ok(block_id)
    }

    fn get_block(&self, block: &String) -> Result<Vec<u8>> {
        let mut block_file = File::create(self.base_dir.join("blocks").join(block))?;
        let mut content = vec![];

        block_file.read_to_end(&mut content)?;

        Ok(content)
    }

    fn commit(&self, node: &String, changes: &[Change]) -> Result<()> {
        DirBuilder::new()
            .recursive(true)
            .create(self.base_dir.join("logs"))?;
        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.base_dir.join("logs").join(node))?;

        for change in changes {
            match change.op {
                Operation::Add => write!(log_file, "A {}\n", change.block)?,
                Operation::Delete => write!(log_file, "D {}\n", change.block)?,
            }
        }
        log_file.flush()?;

        Ok(())
    }
}

fn read_optional_file<P: AsRef<Path>>(path: P) -> Result<Option<Vec<u8>>> {
    match File::open(path) {
        Ok(mut index_file) => {
            let mut content = vec![];

            index_file.read_to_end(&mut content)?;

            Ok(Some(content))
        }
        Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => bail!(err),
    }
}

fn parse_change_log(file_name: &str, path: &Path) -> Result<ChangeLog> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut change_log = ChangeLog::new(file_name);

    for maybe_line in reader.lines() {
        let line = maybe_line?;
        match line.split(' ').collect::<Vec<&str>>().as_slice() {
            ["A", block] => change_log.changes.push(Change::new(Operation::Add, *block)),
            ["D", block] => change_log
                .changes
                .push(Change::new(Operation::Delete, *block)),
            _ => (),
        }
    }

    Ok(change_log)
}

fn generate_id(data: &[u8]) -> Result<String> {
    let mut sha256 = Hasher::new(MessageDigest::sha256())?;

    sha256.update(data)?;

    Ok(HEXLOWER.encode(&sha256.finish()?))
}
