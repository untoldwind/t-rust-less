mod initialize;

use std::{
  collections::{HashMap, VecDeque},
  io,
  io::BufRead,
  io::BufReader,
  io::Read,
  io::Write,
};

pub use initialize::*;

use dropbox_sdk::{
  default_client::UserAuthDefaultClient,
  files::{self, ListFolderError},
  oauth2::Authorization,
  UserAuthClient,
};

use crate::{block_store::generate_block_id, memguard::weak::ZeroingWords};

use super::{BlockStore, Change, ChangeLog, Operation, RingContent, RingId, StoreError, StoreResult};

pub const APP_KEY: &str = "3q0sff542l6r3ly";

pub struct DropboxBlockStore {
  node_id: String,
  name: String,
  client: UserAuthDefaultClient,
}

impl DropboxBlockStore {
  pub fn new(token: &str, name: &str, node_id: &str) -> StoreResult<DropboxBlockStore> {
    let authorization = Authorization::load(APP_KEY.to_string(), token)
      .ok_or_else(|| StoreError::IO("Invalid dropbox token".to_string()))?;
    let client = UserAuthDefaultClient::new(authorization);

    Ok(DropboxBlockStore {
      node_id: node_id.to_string(),
      name: name.to_string(),
      client,
    })
  }

  fn block_path(&self, block_id: &str) -> StoreResult<String> {
    if block_id.len() < 3 {
      return Err(StoreError::InvalidBlock(block_id.to_string()));
    }
    Ok(format!("/{}/blocks/{}/{}", self.name, &block_id[0..2], block_id))
  }

  fn download_change_log(&self, node_id: &str) -> StoreResult<ChangeLog> {
    let (_, maybe_content) = self.download_stream(format!("/{}/logs/{}", self.name, node_id))?;
    match maybe_content {
      Some(content) => Self::parse_change_log(node_id, content),
      _ => Ok(ChangeLog {
        node: node_id.to_string(),
        changes: vec![],
      }),
    }
  }

  #[allow(clippy::type_complexity)]
  fn download_stream(&self, path: String) -> StoreResult<(Option<usize>, Option<Box<dyn Read>>)> {
    match files::download(&self.client, &files::DownloadArg::new(path), None, None)? {
      Ok(result) => {
        let content = result.body.ok_or_else(|| StoreError::IO("No body".to_string()))?;

        Ok((result.content_length.map(|l| l as usize), Some(content)))
      }
      Err(dropbox_sdk::files::DownloadError::Path(_)) => Ok((None, None)),
      Err(dropbox_sdk::files::DownloadError::UnsupportedFile) => Ok((None, None)),
      Err(err) => Err(StoreError::IO(format!("{}", err))),
    }
  }

  fn download(&self, path: String) -> StoreResult<Option<ZeroingWords>> {
    let (content_len, maybe_content) = self.download_stream(path)?;

    if let Some(mut content) = maybe_content {
      let mut buffer = Vec::with_capacity(content_len.unwrap_or(1024usize));
      io::copy(&mut content, &mut buffer)?;

      Ok(Some(ZeroingWords::from(buffer.as_ref())))
    } else {
      Ok(None)
    }
  }

  fn parse_change_log<R: Read>(node_id: &str, content: R) -> StoreResult<ChangeLog> {
    let reader = BufReader::new(content);
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

  fn list_ring_files(&self) -> StoreResult<HashMap<String, (u64, String)>> {
    let mut ring_files: HashMap<String, (u64, String)> = HashMap::new();

    for metadata in list_directory(&self.client, format!("/{}/rings", self.name), false)? {
      if let files::Metadata::File(file_metadata) = metadata? {
        let mut parts = file_metadata.name.split('.');
        let name = parts
          .next()
          .map(str::to_string)
          .unwrap_or_else(|| file_metadata.name.clone());
        let version = parts
          .next()
          .and_then(|version_str| version_str.parse::<u64>().ok())
          .unwrap_or_default();

        if let Some((current, _)) = ring_files.get(&name) {
          if *current > version {
            continue;
          }
        }
        ring_files.insert(name, (version, file_metadata.id));
      }
    }
    Ok(ring_files)
  }
}

impl std::fmt::Debug for DropboxBlockStore {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DropboxBlockStore")
      .field("node_id", &self.node_id)
      .field("name", &self.name)
      .finish()
  }
}

impl BlockStore for DropboxBlockStore {
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
      Some((version, id)) => match self.download(id.clone())? {
        Some(content) => Ok((*version, content)),
        _ => Err(StoreError::InvalidBlock(ring_id.to_string())),
      },
      None => Err(StoreError::InvalidBlock(ring_id.to_string())),
    }
  }

  fn store_ring(&self, ring_id: &str, version: u64, raw: &[u8]) -> StoreResult<()> {
    let path = format!("/{}/rings/{}.{}", self.name, ring_id, version);
    if files::get_metadata(&self.client, &files::GetMetadataArg::new(path.clone()))?.is_ok() {
      return Err(StoreError::Conflict(format!(
        "Ring {} with version {} already exists",
        ring_id, version
      )));
    }
    files::upload(&self.client, &files::UploadArg::new(path), raw)??;
    Ok(())
  }

  fn change_logs(&self) -> StoreResult<Vec<ChangeLog>> {
    list_directory(&self.client, format!("/{}/logs", self.name), false)?
      .filter_map(|metadata| match metadata {
        Ok(files::Metadata::File(f)) => Some(self.download_change_log(&f.name)),
        Err(err) => Some(Err(err)),
        _ => None,
      })
      .collect()
  }

  fn get_index(&self, _index_id: &str) -> StoreResult<Option<ZeroingWords>> {
    // Intentionally left blank. This store is not supposed to be used directly
    Ok(None)
  }

  fn store_index(&self, _index_id: &str, _raw: &[u8]) -> StoreResult<()> {
    // Intentionally left blank. This store is not supposed to be used directly
    Ok(())
  }

  fn add_block(&self, raw: &[u8]) -> StoreResult<String> {
    let block_id = generate_block_id(raw);
    let path = self.block_path(&block_id)?;
    files::upload(&self.client, &files::UploadArg::new(path), raw)??;

    Ok(block_id)
  }

  fn get_block(&self, block: &str) -> StoreResult<ZeroingWords> {
    match self.download(self.block_path(block)?)? {
      Some(content) => Ok(content),
      _ => Err(StoreError::InvalidBlock(block.to_string())),
    }
  }

  fn commit(&self, changes: &[Change]) -> StoreResult<()> {
    let mut change_log = match self.download_change_log(&self.node_id) {
      Ok(change_log) => change_log,
      Err(StoreError::InvalidBlock(_)) => ChangeLog::new(&self.node_id),
      Err(err) => return Err(err),
    };
    if change_log.changes.iter().any(|change| changes.contains(change)) {
      return Err(StoreError::Conflict("Change already committed".to_string()));
    }
    change_log.changes.extend_from_slice(changes);

    let mut buffer = Vec::with_capacity(8192);
    for change in change_log.changes {
      match change.op {
        Operation::Add => writeln!(&mut buffer, "A {}", change.block)?,
        Operation::Delete => writeln!(&mut buffer, "D {}", change.block)?,
      }
    }
    files::upload(
      &self.client,
      &files::UploadArg::new(format!("/{}/logs/{}", self.name, self.node_id)),
      &buffer,
    )??;

    Ok(())
  }

  fn update_change_log(&self, change_log: ChangeLog) -> StoreResult<()> {
    let mut buffer = Vec::with_capacity(8192);
    for change in change_log.changes {
      match change.op {
        Operation::Add => writeln!(&mut buffer, "A {}", change.block)?,
        Operation::Delete => writeln!(&mut buffer, "D {}", change.block)?,
      }
    }
    files::upload(
      &self.client,
      &files::UploadArg::new(format!("/{}/logs/{}", self.name, change_log.node)),
      &buffer,
    )??;

    Ok(())
  }
}

fn list_directory<T: UserAuthClient>(
  client: &T,
  path: String,
  recursive: bool,
) -> StoreResult<DirectoryIterator<'_, T>> {
  let requested_path = if path == "/" { String::new() } else { path };
  let result = match files::list_folder(
    client,
    &files::ListFolderArg::new(requested_path).with_recursive(recursive),
  )? {
    Ok(result) => result,
    Err(ListFolderError::Path(_)) => {
      return Ok(DirectoryIterator {
        client,
        cursor: None,
        buffer: VecDeque::new(),
      })
    }
    Err(err) => return Err(err.into()),
  };

  let cursor = if result.has_more { Some(result.cursor) } else { None };

  Ok(DirectoryIterator {
    client,
    cursor,
    buffer: result.entries.into(),
  })
}

struct DirectoryIterator<'a, T: UserAuthClient> {
  client: &'a T,
  buffer: VecDeque<files::Metadata>,
  cursor: Option<String>,
}

impl<'a, T: UserAuthClient> Iterator for DirectoryIterator<'a, T> {
  type Item = StoreResult<files::Metadata>;

  fn next(&mut self) -> Option<Self::Item> {
    if let Some(entry) = self.buffer.pop_front() {
      Some(Ok(entry))
    } else if let Some(cursor) = self.cursor.take() {
      match files::list_folder_continue(self.client, &files::ListFolderContinueArg::new(cursor)) {
        Ok(Ok(result)) => {
          self.buffer.extend(result.entries);
          if result.has_more {
            self.cursor = Some(result.cursor);
          }
          self.buffer.pop_front().map(Ok)
        }
        Ok(Err(e)) => Some(Err(e.into())),
        Err(e) => Some(Err(e.into())),
      }
    } else {
      None
    }
  }
}
