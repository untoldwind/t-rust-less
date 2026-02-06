use super::{BlockStore, Change, ChangeLog, Operation, RingContent, RingId, StoreError, StoreResult};
use crate::memguard::weak::ZeroingWords;
use log::warn;
use log::{debug, info};
use std::collections::HashMap;
use std::fs::{exists, metadata, read_dir, DirBuilder, File, OpenOptions};
use std::io::prelude::*;
use std::io::{self, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Block store implementation based on a directory of the local file-system.
///
/// This file-layout is structured so that the directory may be shared between multiple clients
/// via rsync, dropbox or similiar tools/services.
///
#[derive(Debug)]
pub struct LocalDirBlockStore {
  node_id: String,
  base_dir: RwLock<PathBuf>,
}

impl LocalDirBlockStore {
  pub fn new<P: Into<PathBuf>>(base_dir_raw: P, node_id: &str) -> StoreResult<LocalDirBlockStore> {
    let base_dir = base_dir_raw.into();
    let md = metadata(&base_dir)?;

    if !md.is_dir() {
      Err(StoreError::InvalidStoreUrl(format!(
        "{} is not a directory",
        base_dir.to_string_lossy()
      )))
    } else {
      info!("Opening local dir store on: {}", base_dir.to_string_lossy());
      Ok(LocalDirBlockStore {
        node_id: node_id.to_string(),
        base_dir: RwLock::new(base_dir),
      })
    }
  }

  fn read_optional_file<P: AsRef<Path>>(path: P) -> StoreResult<Option<ZeroingWords>> {
    debug!("Try reading file: {}", path.as_ref().to_string_lossy());
    match File::open(path) {
      Ok(mut file) => {
        let file_len = file.metadata()?.len() as usize;
        if !file_len.is_multiple_of(8) {
          warn!("File length not aligned to 8 bytes. Probably this is not the file you are looking for.");
        }
        let mut content: ZeroingWords = ZeroingWords::allocate_zeroed_vec(file_len / 8);

        file.read_exact(&mut content)?;

        Ok(Some(content))
      }
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
      Err(err) => Err(err.into()),
    }
  }

  fn parse_change_log(node_id: &str, file: &File) -> StoreResult<ChangeLog> {
    let reader = BufReader::new(file);
    let mut change_log = ChangeLog::new(node_id);

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

  fn block_file(base_dir: &Path, block_id: &str) -> StoreResult<PathBuf> {
    if block_id.len() < 3 {
      return Err(StoreError::InvalidBlock(block_id.to_string()));
    }
    Ok(base_dir.join("blocks").join(&block_id[0..2]).join(block_id))
  }

  fn list_ring_files(&self) -> StoreResult<HashMap<String, (u64, PathBuf)>> {
    match read_dir(self.base_dir.read()?.join("rings")) {
      Ok(ring_dir) => {
        let mut ring_files: HashMap<String, (u64, PathBuf)> = HashMap::new();
        for maybe_entry in ring_dir {
          let entry = maybe_entry?;

          if !entry.metadata()?.is_file() {
            continue;
          }
          if let Some(file_name) = entry.path().file_name() {
            let file_name = file_name.to_string_lossy();
            let mut parts = file_name.split('.');
            let name = parts
              .next()
              .map(str::to_string)
              .unwrap_or_else(|| file_name.to_string());
            let version = parts
              .next()
              .and_then(|version_str| version_str.parse::<u64>().ok())
              .unwrap_or_default();

            if let Some((current, _)) = ring_files.get(&name) {
              if *current > version {
                continue;
              }
            }
            ring_files.insert(name, (version, entry.path().to_owned()));
          }
        }

        Ok(ring_files)
      }
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => Ok(HashMap::new()),
      Err(err) => Err(err.into()),
    }
  }

  fn commit(&self, node_id: &str, changes: &[Change]) -> StoreResult<()> {
    let base_dir = self.base_dir.write()?;
    DirBuilder::new().recursive(true).create(base_dir.join("logs"))?;
    let mut log_file = OpenOptions::new()
      .create(true)
      .write(true)
      .read(true)
      .truncate(false)
      .open(base_dir.join("logs").join(node_id))?;
    let existing = Self::parse_change_log(node_id, &log_file)?;
    log_file.seek(SeekFrom::End(0))?;

    if existing.changes.iter().any(|change| changes.contains(change)) {
      return Err(StoreError::Conflict("Change already committed".to_string()));
    }
    for change in changes {
      match change.op {
        Operation::Add => writeln!(log_file, "A {}", change.block)?,
        Operation::Delete => writeln!(log_file, "D {}", change.block)?,
      }
    }
    log_file.flush()?;
    log_file.sync_all()?;

    Ok(())
  }
}

impl BlockStore for LocalDirBlockStore {
  fn node_id(&self) -> &str {
    &self.node_id
  }

  fn list_ring_ids(&self) -> StoreResult<Vec<RingId>> {
    Ok(
      self
        .list_ring_files()?
        .into_iter()
        .map(|(id, (version, _))| (id, version))
        .collect(),
    )
  }

  fn get_ring(&self, ring_id: &str) -> StoreResult<RingContent> {
    match self.list_ring_files()?.get(ring_id) {
      Some((version, ring_file)) => Ok((
        *version,
        Self::read_optional_file(ring_file)?.ok_or_else(|| StoreError::InvalidBlock(ring_id.to_string()))?,
      )),
      None => Err(StoreError::InvalidBlock(ring_id.to_string())),
    }
  }

  fn store_ring(&self, ring_id: &str, version: u64, raw: &[u8]) -> StoreResult<()> {
    let ring_dir = self.base_dir.write()?.join("rings");
    DirBuilder::new().recursive(true).create(&ring_dir)?;

    let file_name = ring_dir.join(format!("{ring_id}.{version}"));

    if file_name.exists() {
      return Err(StoreError::Conflict(format!(
        "Ring {ring_id} with version {version} already exists",
      )));
    }

    let mut ring_file = File::create(file_name)?;

    ring_file.write_all(raw)?;
    ring_file.flush()?;
    ring_file.sync_all()?;
    Ok(())
  }

  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>> {
    debug!("Try retrieve change logs");
    let base_dir = self.base_dir.read()?;
    let commit_dir = match read_dir(base_dir.join("logs")) {
      Ok(dir) => dir,
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => return Ok(vec![]),
      Err(err) => return Err(err.into()),
    };
    let mut change_logs: Vec<ChangeLog> = vec![];

    for maybe_entry in commit_dir {
      let entry = maybe_entry?;

      if !entry.metadata()?.is_file() {
        continue;
      }
      let file = File::open(entry.path())?;

      change_logs.push(Self::parse_change_log(&entry.file_name().to_string_lossy(), &file)?);
    }

    Ok(change_logs)
  }

  fn get_index(&self, index_id: &str) -> StoreResult<Option<ZeroingWords>> {
    debug!("Try getting index  {index_id}");
    let base_dir = self.base_dir.read()?;
    Self::read_optional_file(base_dir.join("indexes").join(&self.node_id).join(index_id))
  }

  fn store_index(&self, index_id: &str, raw: &[u8]) -> StoreResult<()> {
    debug!("Try storing index  {index_id}");
    let base_dir = self.base_dir.write()?;
    let index_file_path = base_dir.join("indexes").join(&self.node_id).join(index_id);
    DirBuilder::new()
      .recursive(true)
      .create(index_file_path.parent().unwrap())?;
    let mut index_file = File::create(index_file_path)?;

    index_file.write_all(raw)?;
    index_file.flush()?;

    Ok(())
  }

  fn insert_block(&self, block_id: &str, node_id: &str, raw: &[u8]) -> StoreResult<()> {
    {
      let base_dir = self.base_dir.write()?;
      let block_file_path = Self::block_file(&base_dir, block_id)?;

      if exists(&block_file_path)? {
        return Err(StoreError::Conflict(block_id.to_string()));
      }

      DirBuilder::new()
        .recursive(true)
        .create(block_file_path.parent().unwrap())?;
      let mut block_file = File::create(block_file_path)?;

      block_file.write_all(raw)?;
      block_file.flush()?;
      block_file.sync_all()?;
    }

    self.commit(
      node_id,
      &[Change {
        op: Operation::Add,
        block: block_id.to_string(),
      }],
    )?;

    Ok(())
  }

  fn get_block(&self, block_id: &str) -> StoreResult<ZeroingWords> {
    let base_dir = self.base_dir.read()?;
    let block_file_path = Self::block_file(&base_dir, block_id)?;

    Self::read_optional_file(block_file_path)?.ok_or_else(|| StoreError::InvalidBlock(block_id.to_string()))
  }

  fn check_block(&self, block_id: &str) -> StoreResult<bool> {
    let base_dir = self.base_dir.read()?;
    let block_file_path = Self::block_file(&base_dir, block_id)?;

    Ok(exists(block_file_path)?)
  }
}
