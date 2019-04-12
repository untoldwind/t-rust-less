use super::{BlockStore, Change, ChangeLog, Operation, StoreError, StoreResult};
use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use std::fs::{read_dir, DirBuilder, File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Block store implementation based on a directory of the local file-system.
///
/// This file-layout is structured so that the directory may be shared between multiple clients
/// via rsync, dropbox or similiar tools/services.
///
#[derive(Debug)]
pub struct LocalDirBlockStore {
  base_dir: RwLock<PathBuf>,
}

impl LocalDirBlockStore {
  pub fn new<P: Into<PathBuf>>(base_dir: P) -> LocalDirBlockStore {
    LocalDirBlockStore {
      base_dir: RwLock::new(base_dir.into()),
    }
  }

  fn read_optional_file<P: AsRef<Path>>(path: P) -> StoreResult<Option<Vec<u8>>> {
    match File::open(path) {
      Ok(mut index_file) => {
        let mut content = vec![];

        index_file.read_to_end(&mut content)?;

        Ok(Some(content))
      }
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
      Err(err) => Err(err.into()),
    }
  }

  fn parse_change_log(file_name: &str, path: &Path) -> StoreResult<ChangeLog> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut change_log = ChangeLog::new(file_name);

    for maybe_line in reader.lines() {
      let line = maybe_line?;
      match line.split(' ').collect::<Vec<&str>>().as_slice() {
        ["A", block] => change_log.changes.push(Change::new(Operation::Add, *block)),
        ["D", block] => change_log.changes.push(Change::new(Operation::Delete, *block)),
        _ => (),
      }
    }

    Ok(change_log)
  }

  fn generate_id(data: &[u8]) -> String {
    let mut hasher = Sha256::new();

    hasher.input(data);

    HEXLOWER.encode(&hasher.result())
  }

  fn block_file(base_dir: &PathBuf, block_id: &str) -> StoreResult<PathBuf> {
    if block_id.len() < 3 {
      return Err(StoreError::InvalidBlock(block_id.to_string()));
    }
    Ok(base_dir.join("blocks").join(&block_id[0..2]).join(block_id))
  }
}

impl BlockStore for LocalDirBlockStore {
  fn get_ring(&self) -> StoreResult<Option<Vec<u8>>> {
    let base_dir = self.base_dir.read()?;
    Self::read_optional_file(base_dir.join("ring"))
  }

  fn store_ring(&mut self, raw: &[u8]) -> StoreResult<()> {
    let maybe_current = self.get_ring()?;
    let base_dir = self.base_dir.write()?;

    match maybe_current {
      Some(current) => {
        let mut backup_file = File::create(base_dir.join("ring.bak"))?;

        backup_file.write_all(&current)?;
        backup_file.flush()?;
        backup_file.sync_all()?;
      }
      _ => (),
    }

    let mut ring_file = File::create(base_dir.join("ring"))?;

    ring_file.write_all(raw)?;
    ring_file.flush()?;
    ring_file.sync_all()?;
    Ok(())
  }

  fn get_public_ring(&self) -> StoreResult<Option<Vec<u8>>> {
    let base_dir = self.base_dir.read()?;
    Self::read_optional_file(base_dir.join("ring.pub"))
  }

  fn store_public_ring(&mut self, raw: &[u8]) -> StoreResult<()> {
    let maybe_current = self.get_public_ring()?;
    let base_dir = self.base_dir.write()?;

    match maybe_current {
      Some(current) => {
        let mut backup_file = File::create(base_dir.join("ring.pub,bak"))?;

        backup_file.write_all(&current)?;
        backup_file.flush()?;
        backup_file.sync_all()?;
      }
      _ => (),
    }

    let mut ring_file = File::create(base_dir.join("ring.pub"))?;

    ring_file.write_all(raw)?;
    ring_file.flush()?;
    ring_file.sync_all()?;
    Ok(())
  }

  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>> {
    let base_dir = self.base_dir.read()?;
    let commit_dir = read_dir(base_dir.join("logs"))?;
    let mut change_logs: Vec<ChangeLog> = vec![];

    for maybe_entry in commit_dir {
      let entry = maybe_entry?;

      if !entry.metadata()?.is_file() {
        continue;
      }
      change_logs.push(Self::parse_change_log(
        &entry.file_name().to_string_lossy(),
        &entry.path(),
      )?);
    }

    Ok(change_logs)
  }

  fn get_index(&self, node: &str) -> StoreResult<Option<Vec<u8>>> {
    let base_dir = self.base_dir.read()?;
    Self::read_optional_file(base_dir.join("indexes").join(node))
  }

  fn store_index(&mut self, node: &str, raw: &[u8]) -> StoreResult<()> {
    let base_dir = self.base_dir.write()?;
    DirBuilder::new().recursive(true).create(base_dir.join("indexes"))?;
    let mut index_file = File::create(base_dir.join("indexes").join(node))?;

    index_file.write_all(raw)?;
    index_file.flush()?;

    Ok(())
  }

  fn add_block(&mut self, raw: &[u8]) -> StoreResult<String> {
    let base_dir = self.base_dir.write()?;
    let block_id = Self::generate_id(raw);
    let block_file_path = Self::block_file(&base_dir, &block_id)?;

    DirBuilder::new()
      .recursive(true)
      .create(block_file_path.parent().unwrap())?;
    let mut block_file = File::create(block_file_path)?;

    block_file.write_all(raw)?;
    block_file.flush()?;

    Ok(block_id)
  }

  fn get_block(&self, block: &str) -> StoreResult<Vec<u8>> {
    let base_dir = self.base_dir.read()?;
    let block_file_path = Self::block_file(&base_dir, &block)?;

    Self::read_optional_file(&block_file_path)?.ok_or_else(|| StoreError::InvalidBlock(block.to_string()))
  }

  fn commit(&mut self, node: &str, changes: &[Change]) -> StoreResult<()> {
    let base_dir = self.base_dir.write()?;
    DirBuilder::new().recursive(true).create(base_dir.join("logs"))?;
    let mut log_file = OpenOptions::new()
      .create(true)
      .append(true)
      .open(base_dir.join("logs").join(node))?;

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
