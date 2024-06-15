use byteorder::{ByteOrder, LittleEndian};
use log::{debug, info, warn};
use std::{
  collections::HashMap,
  fs::{metadata, read_dir, File},
  io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
  path::{Path, PathBuf},
  sync::RwLock,
};

use crate::memguard::weak::ZeroingWords;

use super::{BlockStore, Change, ChangeLog, Operation, StoreError, StoreResult};

#[derive(Debug)]
pub struct LocalWalBlockStore {
  node_id: String,
  base_dir: RwLock<PathBuf>,
}

impl LocalWalBlockStore {
  pub fn new<P: Into<PathBuf>>(base_dir_raw: P, node_id: &str) -> StoreResult<LocalWalBlockStore> {
    let base_dir = base_dir_raw.into();
    let md = metadata(&base_dir)?;

    if !md.is_dir() {
      Err(StoreError::InvalidStoreUrl(format!(
        "{} is not a directory",
        base_dir.to_string_lossy()
      )))
    } else {
      info!("Opening local wal store on: {}", base_dir.to_string_lossy());
      Ok(LocalWalBlockStore {
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
        if file_len % 8 != 0 {
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

  fn list_ring_files(&self) -> StoreResult<HashMap<String, (u64, PathBuf)>> {
    let mut ring_files: HashMap<String, (u64, PathBuf)> = HashMap::new();
    for maybe_entry in read_dir(self.base_dir.read()?.as_path())? {
      let entry = maybe_entry?;

      if !entry.metadata()?.is_file() {
        continue;
      }
      if let Some(file_name) = entry.file_name().to_str() {
        if !file_name.ends_with(".ring") {
          continue;
        }
        let (name, version) = match file_name.trim_end_matches(".ring").split_once('.') {
          Some(parts) => parts,
          _ => continue,
        };
        let version = match version.parse::<u64>() {
          Ok(version) => version,
          _ => continue,
        };

        if let Some((current, _)) = ring_files.get(name) {
          if *current > version {
            continue;
          }
        }
        ring_files.insert(name.to_string(), (version, entry.path().to_owned()));
      }
    }
    Ok(ring_files)
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
}

impl BlockStore for LocalWalBlockStore {
  fn node_id(&self) -> &str {
    &self.node_id
  }

  fn list_ring_ids(&self) -> StoreResult<Vec<super::RingId>> {
    Ok(
      self
        .list_ring_files()?
        .into_iter()
        .map(|(id, (version, _))| (id, version))
        .collect(),
    )
  }

  fn get_ring(&self, ring_id: &str) -> StoreResult<super::RingContent> {
    match self.list_ring_files()?.get(ring_id) {
      Some((version, ring_file)) => Ok((
        *version,
        Self::read_optional_file(ring_file)?.ok_or_else(|| StoreError::InvalidBlock(ring_id.to_string()))?,
      )),
      None => Err(StoreError::InvalidBlock(ring_id.to_string())),
    }
  }

  fn store_ring(&self, ring_id: &str, version: u64, raw: &[u8]) -> StoreResult<()> {
    let base_dir = self.base_dir.write()?;
    let file_name = base_dir.join(format!("{}.{}.ring", ring_id, version));

    if file_name.exists() {
      return Err(StoreError::Conflict(format!(
        "Ring {} with version {} already exists",
        ring_id, version
      )));
    }

    let mut ring_file = File::create(file_name)?;

    ring_file.write_all(raw)?;
    ring_file.flush()?;
    ring_file.sync_all()?;
    Ok(())
  }

  fn change_logs(&self) -> StoreResult<Vec<super::ChangeLog>> {
    debug!("Try retrieve change logs");
    let mut change_logs: Vec<ChangeLog> = vec![];

    for maybe_entry in read_dir(self.base_dir.read()?.as_path())? {
      let entry = maybe_entry?;

      if !entry.metadata()?.is_file() {
        continue;
      }
      if let Some(file_name) = entry.file_name().to_str() {
        if !file_name.ends_with(".commits") {
          continue;
        }
        let file = File::open(entry.path())?;

        change_logs.push(Self::parse_change_log(file_name.trim_end_matches(".commits"), &file)?);
      }
    }

    Ok(change_logs)
  }

  fn get_index(&self, index_id: &str) -> StoreResult<Option<crate::memguard::weak::ZeroingWords>> {
    debug!("Try getting index  {}", index_id);
    let base_dir = self.base_dir.read()?;
    Self::read_optional_file(base_dir.join(format!("{}.{}.index", self.node_id, index_id)))
  }

  fn store_index(&self, index_id: &str, raw: &[u8]) -> StoreResult<()> {
    debug!("Try storing index  {}", index_id);
    let base_dir = self.base_dir.write()?;
    let index_file_path = base_dir.join(format!("{}.{}.index", self.node_id, index_id));
    let mut index_file = File::create(index_file_path)?;

    index_file.write_all(raw)?;
    index_file.flush()?;

    Ok(())
  }

  fn add_block(&self, raw: &[u8]) -> StoreResult<String> {
    let base_dir = self.base_dir.write()?;
    let block_file_path = base_dir.join(format!("{}.blocks", self.node_id));

    let block_id = match metadata(&block_file_path) {
      Ok(metadata) => format!("{}:{}", self.node_id, metadata.len()),
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => format!("{}:0", self.node_id),
      Err(err) => return Err(err.into()),
    };

    let mut block_file = File::options().create(true).append(true).open(block_file_path)?;

    let mut chunk_size = [0u8; 8];
    LittleEndian::write_u64(&mut chunk_size, raw.len() as u64);
    block_file.write_all(&chunk_size)?;
    block_file.write_all(raw)?;
    block_file.flush()?;
    block_file.sync_all()?;

    Ok(block_id)
  }

  fn get_block(&self, block: &str) -> StoreResult<crate::memguard::weak::ZeroingWords> {
    let base_dir = self.base_dir.read()?;
    let (node_id, offset) = block
      .split_once(':')
      .ok_or_else(|| StoreError::InvalidBlock(block.to_string()))?;
    let offset = offset
      .parse::<u64>()
      .map_err(|_| StoreError::InvalidBlock(block.to_string()))?;

    let mut block_file = File::open(base_dir.join(format!("{}.blocks", node_id)))?;
    block_file.seek(SeekFrom::Start(offset))?;
    let mut chunk_size = [0u8; 8];
    block_file.read_exact(&mut chunk_size)?;
    let chunk_size = LittleEndian::read_u64(&chunk_size) as usize;
    let mut content: ZeroingWords = ZeroingWords::allocate_zeroed_vec(chunk_size / 8);
    block_file.read_exact(&mut content)?;

    Ok(content)
  }

  fn commit(&self, changes: &[super::Change]) -> StoreResult<()> {
    let base_dir = self.base_dir.write()?;
    let mut log_file = File::options()
      .create(true)
      .write(true)
      .read(true)
      .truncate(false)
      .open(base_dir.join(format!("{}.commits", self.node_id)))?;
    let existing = Self::parse_change_log(&self.node_id, &log_file)?;
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

  fn update_change_log(&self, change_log: super::ChangeLog) -> StoreResult<()> {
    let base_dir = self.base_dir.write()?;
    let mut change_log_file = File::create(base_dir.join(format!("{}.commits", self.node_id)))?;

    for change in change_log.changes {
      match change.op {
        Operation::Add => writeln!(change_log_file, "A {}", change.block)?,
        Operation::Delete => writeln!(change_log_file, "D {}", change.block)?,
      }
    }
    change_log_file.flush()?;
    change_log_file.sync_all()?;

    Ok(())
  }
}
