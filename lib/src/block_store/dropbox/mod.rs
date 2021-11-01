mod initialize;

use std::{collections::VecDeque, io, io::BufRead, io::BufReader, io::Read, io::Write};

pub use initialize::*;

use dropbox_sdk::{default_client::UserAuthDefaultClient, files, oauth2::Authorization, UserAuthClient};

use crate::{block_store::generate_block_id, memguard::weak::ZeroingWords};

use super::{BlockStore, Change, ChangeLog, Operation, StoreError, StoreResult};

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
    let (_, content) = self.download_stream(format!("/{}/logs/{}", self.name, node_id))?;
    Self::parse_change_log(node_id, content)
  }

  fn download_stream(&self, path: String) -> StoreResult<(Option<usize>, Box<dyn Read>)> {
    let result = match files::download(&self.client, &files::DownloadArg::new(path.clone()), None, None)? {
      Ok(result) => result,
      Err(dropbox_sdk::files::DownloadError::Path(_)) => return Err(StoreError::InvalidBlock(path)),
      Err(dropbox_sdk::files::DownloadError::UnsupportedFile) => return Err(StoreError::InvalidBlock(path)),
      Err(err) => return Err(StoreError::IO(format!("{}", err))),
    };
    let content = result.body.ok_or_else(|| StoreError::IO("No body".to_string()))?;

    Ok((result.content_length.map(|l| l as usize), content))
  }

  fn download(&self, path: String) -> StoreResult<ZeroingWords> {
    let (content_len, mut content) = self.download_stream(path)?;
    let mut buffer = Vec::with_capacity(content_len.unwrap_or(1024usize));

    io::copy(&mut content, &mut buffer)?;

    Ok(ZeroingWords::from(buffer.as_ref()))
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
}

impl BlockStore for DropboxBlockStore {
  fn node_id(&self) -> &str {
    &self.node_id
  }

  fn list_ring_ids(&self) -> StoreResult<Vec<String>> {
    list_directory(&self.client, format!("/{}/rings", self.name), false)?
      .filter_map(|metadata| match metadata {
        Ok(files::Metadata::File(f)) => Some(Ok(f.name)),
        Err(err) => Some(Err(err)),
        _ => None,
      })
      .collect()
  }

  fn get_ring(&self, ring_id: &str) -> StoreResult<ZeroingWords> {
    self.download(format!("/{}/rings/{}", self.name, ring_id))
  }

  fn store_ring(&self, ring_id: &str, raw: &[u8]) -> StoreResult<()> {
    let path = format!("/{}/rings/{}", self.name, ring_id);
    files::upload(&self.client, &files::CommitInfo::new(path), raw)??;
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
    files::upload(&self.client, &files::CommitInfo::new(path), raw)??;

    Ok(block_id)
  }

  fn get_block(&self, block: &str) -> StoreResult<ZeroingWords> {
    self.download(self.block_path(block)?)
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
      &files::CommitInfo::new(format!("/{}/logs/{}", self.name, self.node_id)),
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
      &files::CommitInfo::new(format!("/{}/logs/{}", self.name, change_log.node)),
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
  assert!(path.starts_with('/'), "path needs to be absolute (start with a '/')");
  let requested_path = if path == "/" { String::new() } else { path };
  let result = files::list_folder(
    client,
    &files::ListFolderArg::new(requested_path).with_recursive(recursive),
  )??;

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
          self.buffer.extend(result.entries.into_iter());
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
