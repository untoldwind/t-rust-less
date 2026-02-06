use byteorder::{ByteOrder, LittleEndian};
use hashlink::LinkedHashMap;
use log::{debug, info, warn};
use std::{
  collections::HashMap,
  fs::{metadata, read_dir, File},
  io::{self, Read, Seek, SeekFrom, Write},
  path::{Path, PathBuf},
  sync::RwLock,
};

use crate::memguard::weak::ZeroingWords;

use super::{BlockStore, Change, ChangeLog, Operation, StoreError, StoreResult};

#[derive(Debug)]
struct WalFileRef {
  node_id: String,
  pos: u64,
}

#[derive(Debug)]
pub struct LocalWalBlockStore {
  node_id: String,
  base_dir: RwLock<PathBuf>,
  block_refs: RwLock<LinkedHashMap<String, WalFileRef>>,
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
      let block_refs = Self::scan_wal_files(&base_dir)?;
      Ok(LocalWalBlockStore {
        node_id: node_id.to_string(),
        base_dir: RwLock::new(base_dir),
        block_refs: RwLock::new(block_refs),
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

  fn scan_wal_files(base_dir: &Path) -> StoreResult<LinkedHashMap<String, WalFileRef>> {
    let mut result = LinkedHashMap::new();
    for maybe_entry in read_dir(base_dir)? {
      let entry = maybe_entry?;

      if !entry.metadata()?.is_file() {
        continue;
      }
      if let Some(file_name) = entry.file_name().to_str() {
        if !file_name.ends_with(".blocks") {
          continue;
        }

        let node_id = file_name.trim_end_matches(".blocks");

        let mut block_file = File::options().read(true).open(entry.path())?;

        let mut header = [0u8; 8];
        let mut pos = 0u64;

        while block_file.read_exact(&mut header).is_ok() {
          let data_size = LittleEndian::read_u32(&header);
          let blockid_size = LittleEndian::read_u32(&header[4..]);
          let mut block_id = vec![0u8; blockid_size as usize];
          block_file.read_exact(&mut block_id)?;
          block_file.seek_relative(data_size as i64)?;

          result.insert(
            String::from_utf8(block_id).unwrap(),
            WalFileRef {
              node_id: node_id.to_string(),
              pos,
            },
          );

          pos += (data_size + blockid_size + 8) as u64;
        }
      }
    }

    Ok(result)
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
    let file_name = base_dir.join(format!("{ring_id}.{version}.ring"));

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

  fn change_logs(&self) -> StoreResult<Vec<super::ChangeLog>> {
    debug!("Try retrieve change logs");
    let mut changes_by_node = HashMap::new();
    for (block_id, file_ref) in self.block_refs.read()?.iter() {
      let changes: &mut Vec<Change> = match changes_by_node.get_mut(&file_ref.node_id) {
        Some(change_log) => change_log,
        _ => {
          changes_by_node.insert(file_ref.node_id.to_string(), vec![]);
          changes_by_node.get_mut(&file_ref.node_id).unwrap()
        }
      };
      changes.push(Change {
        op: Operation::Add,
        block: block_id.to_string(),
      });
    }

    let mut change_logs: Vec<ChangeLog> = vec![];

    for (node, changes) in changes_by_node.into_iter() {
      change_logs.push(ChangeLog { node, changes });
    }

    Ok(change_logs)
  }

  fn get_index(&self, index_id: &str) -> StoreResult<Option<crate::memguard::weak::ZeroingWords>> {
    debug!("Try getting index  {index_id}");
    let base_dir = self.base_dir.read()?;
    Self::read_optional_file(base_dir.join(format!("{}.{}.index", self.node_id, index_id)))
  }

  fn store_index(&self, index_id: &str, raw: &[u8]) -> StoreResult<()> {
    debug!("Try storing index  {index_id}");
    let base_dir = self.base_dir.write()?;
    let index_file_path = base_dir.join(format!("{}.{}.index", self.node_id, index_id));
    let mut index_file = File::create(index_file_path)?;

    index_file.write_all(raw)?;
    index_file.flush()?;

    Ok(())
  }

  fn insert_block(&self, block_id: &str, node_id: &str, raw: &[u8]) -> StoreResult<()> {
    if self.block_refs.read()?.contains_key(block_id) {
      return Err(StoreError::Conflict(block_id.to_string()));
    }

    let base_dir = self.base_dir.write()?;
    let block_file_path = base_dir.join(format!("{}.blocks", node_id));

    let pos = match metadata(&block_file_path) {
      Ok(metadata) => metadata.len(),
      Err(ref err) if err.kind() == io::ErrorKind::NotFound => 0,
      Err(err) => return Err(err.into()),
    };

    let mut block_file = File::options().create(true).append(true).open(block_file_path)?;

    let raw_block_id = block_id.as_bytes();
    let mut header = [0u8; 8];
    LittleEndian::write_u32(&mut header, raw.len() as u32);
    LittleEndian::write_u32(&mut header[4..], raw_block_id.len() as u32);
    block_file.write_all(&header)?;
    block_file.write_all(raw_block_id)?;
    block_file.write_all(raw)?;
    block_file.flush()?;
    block_file.sync_all()?;

    self.block_refs.write()?.insert(
      block_id.to_string(),
      WalFileRef {
        node_id: self.node_id.to_string(),
        pos,
      },
    );

    Ok(())
  }

  fn get_block(&self, block_id: &str) -> StoreResult<crate::memguard::weak::ZeroingWords> {
    let base_dir = self.base_dir.read()?;
    let (file_path, offset) = match self.block_refs.read()?.get(block_id) {
      Some(WalFileRef { node_id, pos, .. }) => (base_dir.join(format!("{node_id}.blocks")), *pos),
      _ => return Err(StoreError::InvalidBlock(block_id.to_string())),
    };

    let mut block_file = File::open(file_path)?;
    block_file.seek(SeekFrom::Start(offset))?;
    let mut header = [0u8; 8];
    block_file.read_exact(&mut header)?;
    let data_size = LittleEndian::read_u32(&header) as usize;
    if !data_size.is_multiple_of(8) {
      warn!("Data length not aligned to 8 bytes. Probably this is not the file you are looking for.");
    }
    let block_id_size = LittleEndian::read_u32(&header[4..]) as i64;
    block_file.seek_relative(block_id_size)?;
    let mut content: ZeroingWords = ZeroingWords::allocate_zeroed_vec(data_size / 8);
    block_file.read_exact(&mut content)?;

    Ok(content)
  }

  fn check_block(&self, block_id: &str) -> StoreResult<bool> {
    Ok(self.block_refs.read()?.contains_key(block_id))
  }
}
