use crate::api::{SecretEntry, SecretEntryMatch, SecretListFilter, SecretType, SecretVersion};
use crate::block_store::{Change, ChangeLog, Operation};
use crate::memguard::weak::{ZeroingHHeapAllocator, ZeroingString, ZeroingStringExt};
use crate::memguard::SecretWords;
use crate::secrets_store::SecretStoreResult;
use crate::secrets_store_capnp::index;
use capnp::{message, serialize, text_list};
use chrono::prelude::*;
use std::collections::{HashMap, HashSet};

pub struct Index {
  data: SecretWords,
  heads: HashMap<String, Change>,
}

impl Index {
  fn from_raw(raw: &mut [u8]) -> SecretStoreResult<Index> {
    let data = SecretWords::from(raw);
    let heads = Self::read_current_heads(&data)?;

    Ok(Index { data, heads })
  }

  pub fn filter_entries(&self, filter: &SecretListFilter) -> SecretStoreResult<Vec<SecretEntryMatch>> {
    let data_borrow = self.data.borrow();
    let reader = serialize::read_message_from_words(&data_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut matches = Vec::new();

    for entry in index.get_entries()? {
      if let Some(entry_match) = Self::match_entry(entry, filter)? {
        matches.push(entry_match);
      }
    }

    Ok(matches)
  }

  pub fn process_change_logs<F>(&mut self, change_logs: &[ChangeLog], version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let (new_heads, added_versions, deleted_blocks) = self.collect_changes(change_logs, &version_accessor)?;

    if added_versions.is_empty() && deleted_blocks.is_empty() {
      return Ok(false); // No change that affects us
    }

    let to_keep = self.collect_entries_to_keep(&deleted_blocks)?;
    let additions = added_versions.keys().filter(|id| !to_keep.contains(*id)).count();
    let mut index_message = message::Builder::new(ZeroingHHeapAllocator::new());
    {
      let index_borrow = self.data.borrow();
      let reader = serialize::read_message_from_words(&index_borrow, message::ReaderOptions::new())?;
      let old_index = reader.get_root::<index::Reader>()?;
      let mut new_index = index_message.init_root::<index::Builder>();

      Self::update_heads(new_index.reborrow(), &new_heads);
      let mut entry_pos = 0;
      let mut new_entries = new_index.init_entries((to_keep.len() + additions) as u32);

      for old_entry in old_index.get_entries()? {
        let secret_id = old_entry.get_id()?;
        if !to_keep.contains(secret_id) {
          continue;
        }
        let new_block_ids = Self::merge_block_ids(
          old_entry.reborrow(),
          added_versions.get(secret_id).map(|m| m.keys()),
          &deleted_blocks,
        )?;

        if deleted_blocks.contains(old_entry.get_current_block_id()?) {
          Self::recreate_entry(
            new_entries.reborrow().get(entry_pos),
            new_block_ids,
            added_versions.get(secret_id),
            &version_accessor,
          )?;
          entry_pos += 1;
        } else {
          new_entries.set_with_caveats(entry_pos, old_entry)?;
          Self::update_entry(
            new_entries.reborrow().get(entry_pos),
            new_block_ids,
            added_versions.get(secret_id),
          )?;
          entry_pos += 1;
        }
      }
      for (secret_id, added_version) in added_versions {
        if to_keep.contains(&secret_id) {
          continue;
        }
        Self::update_entry(
          new_entries.reborrow().get(entry_pos),
          added_version.keys().cloned().collect(),
          Some(&added_version),
        )?;
        entry_pos += 1;
      }
    }

    self.data = SecretWords::from(serialize::write_message_to_words(&index_message));
    self.heads = new_heads;

    Ok(true)
  }

  fn collect_entries_to_keep(&self, deleted_blocks: &HashSet<String>) -> SecretStoreResult<HashSet<String>> {
    let data_borrow = self.data.borrow();
    let reader = serialize::read_message_from_words(&data_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut to_keep = HashSet::new();

    for entry in index.get_entries()? {
      let secret_id = entry.get_id()?;
      let mut remainging_count = 0;
      for maybe_block_id in entry.get_block_ids()? {
        let block_id = maybe_block_id?;
        if !deleted_blocks.contains(block_id) {
          remainging_count += 1
        }
      }
      if remainging_count > 0 {
        to_keep.insert(secret_id.to_string());
      }
    }

    Ok(to_keep)
  }

  fn collect_changes<F>(
    &mut self,
    change_logs: &[ChangeLog],
    version_accessor: F,
  ) -> SecretStoreResult<(
    HashMap<String, Change>,
    HashMap<String, HashMap<String, SecretVersion>>,
    HashSet<String>,
  )>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let mut new_heads = HashMap::with_capacity(change_logs.len());
    let mut added_versions = HashMap::new();
    let mut deleted_blocks = HashSet::new();

    for change_log in change_logs {
      let changes = change_log.changes_since(self.heads.get(&change_log.node));

      for change in changes {
        match change.op {
          Operation::Add => {
            if let Some(secret_version) = version_accessor(&change.block)? {
              let secret_id = secret_version.secret_id.clone();
              let mut by_blocks = added_versions.remove(&secret_id).unwrap_or_else(|| HashMap::new());
              by_blocks.insert(change.block.clone(), secret_version);
              added_versions.insert(secret_id, by_blocks);
            }
          }
          Operation::Delete => {
            deleted_blocks.insert(change.block.clone());
          }
        }
      }

      if let Some(last) = change_log.changes.last() {
        new_heads.insert(change_log.node.clone(), last.clone());
      }
    }

    // Note there are usually more added blocks then deleted, so it is reasonable to do it this way
    for deleted_block in &deleted_blocks {
      for by_block in added_versions.values_mut() {
        by_block.remove(deleted_block);
      }
    }

    Ok((new_heads, added_versions, deleted_blocks))
  }

  fn read_current_heads(index_data: &SecretWords) -> SecretStoreResult<HashMap<String, Change>> {
    let index_borrow = index_data.borrow();
    let reader = serialize::read_message_from_words(&index_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut heads = HashMap::with_capacity(index.get_heads()?.len() as usize);

    for head in index.get_heads()? {
      let node_id = head.get_node_id()?.to_string();
      let op = match head.get_operation()? {
        index::HeadOperation::Add => Operation::Add,
        index::HeadOperation::Delete => Operation::Delete,
      };
      let block = head.get_block_id()?.to_string();
      heads.insert(node_id, Change { op, block });
    }

    Ok(heads)
  }

  fn update_heads(index: index::Builder, heads: &HashMap<String, Change>) {
    let mut new_heads = index.init_heads(heads.len() as u32);

    for (idx, (node_id, change)) in heads.into_iter().enumerate() {
      let mut new_head = new_heads.reborrow().get(idx as u32);

      new_head.set_node_id(&node_id);
      match change.op {
        Operation::Add => new_head.set_operation(index::HeadOperation::Add),
        Operation::Delete => new_head.set_operation(index::HeadOperation::Delete),
      }
      new_head.set_block_id(&change.block);
    }
  }

  fn merge_block_ids<I, S>(
    old_entry: index::entry::Reader,
    maybe_added_blocks: Option<I>,
    deleted_blocks: &HashSet<String>,
  ) -> SecretStoreResult<HashSet<String>>
  where
    I: Iterator<Item = S>,
    S: AsRef<str>,
  {
    let mut block_ids = HashSet::new();

    for maybe_block_id in old_entry.get_block_ids()? {
      let block_id = maybe_block_id?;

      if !deleted_blocks.contains(block_id) {
        block_ids.insert(block_id.to_string());
      }
    }
    if let Some(added_blocks) = maybe_added_blocks {
      for block_id in added_blocks {
        block_ids.insert(block_id.as_ref().to_string());
      }
    }
    Ok(block_ids)
  }

  fn recreate_entry<F>(
    mut new_entry: index::entry::Builder,
    mut new_block_ids: HashSet<String>,
    maybe_added_versions: Option<&HashMap<String, SecretVersion>>,
    version_accessor: F,
  ) -> SecretStoreResult<()>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    for block_id in new_block_ids.clone() {
      let maybe_version = match maybe_added_versions.and_then(|a| a.get(&block_id)) {
        None => version_accessor(&block_id)?,
        Some(version) => Some(version.clone()),
      };
      match maybe_version {
        Some(version) => {
          if new_entry.reborrow().get_current_block_id()?.is_empty()
            || new_entry.reborrow().get_timestamp() < version.timestamp.timestamp_millis()
          {
            Self::overwrite_entry(new_entry.reborrow(), &block_id, &version)?;
          }
        }
        None => {
          new_block_ids.remove(&block_id);
        }
      }
    }

    Self::set_text_list(
      new_entry.reborrow().init_block_ids(new_block_ids.len() as u32),
      &new_block_ids,
    )?;

    Ok(())
  }

  fn update_entry(
    mut new_entry: index::entry::Builder,
    mut new_block_ids: HashSet<String>,
    maybe_added_versions: Option<&HashMap<String, SecretVersion>>,
  ) -> SecretStoreResult<()> {
    for block_id in new_block_ids.clone() {
      let maybe_version = maybe_added_versions.and_then(|a| a.get(&block_id));
      match maybe_version {
        Some(version) => {
          if new_entry.reborrow().get_current_block_id()?.is_empty()
            || new_entry.reborrow().get_timestamp() < version.timestamp.timestamp_millis()
          {
            Self::overwrite_entry(new_entry.reborrow(), &block_id, version)?;
          }
        }
        None => {
          new_block_ids.remove(&block_id);
        }
      }
    }

    Self::set_text_list(
      new_entry.reborrow().init_block_ids(new_block_ids.len() as u32),
      &new_block_ids,
    )?;

    Ok(())
  }

  fn overwrite_entry(
    mut new_entry: index::entry::Builder,
    block_id: &str,
    version: &SecretVersion,
  ) -> SecretStoreResult<()> {
    new_entry.set_id(&version.secret_id);
    new_entry.set_timestamp(version.timestamp.timestamp_millis());
    new_entry.set_name(&version.name);
    match version.secret_type {
      SecretType::Login => new_entry.set_type(index::SecretType::Login),
      SecretType::Licence => new_entry.set_type(index::SecretType::Licence),
      SecretType::Note => new_entry.set_type(index::SecretType::Note),
      SecretType::Wlan => new_entry.set_type(index::SecretType::Wlan),
      SecretType::Password => new_entry.set_type(index::SecretType::Password),
      SecretType::Other => new_entry.set_type(index::SecretType::Other),
    }
    Self::set_text_list(new_entry.reborrow().init_tags(version.tags.len() as u32), &version.tags)?;
    Self::set_text_list(new_entry.reborrow().init_urls(version.urls.len() as u32), &version.urls)?;
    new_entry.set_deleted(version.deleted);
    new_entry.set_current_block_id(block_id);

    Ok(())
  }

  fn match_entry(
    entry: index::entry::Reader,
    filter: &SecretListFilter,
  ) -> SecretStoreResult<Option<SecretEntryMatch>> {
    if filter.deleted != entry.get_deleted() {
      return Ok(None)
    }

    let (name_score, name_highlights) = match &filter.name {
      Some(name_filter) => match  sublime_fuzzy::best_match(name_filter, entry.get_name()?) {
        Some(fuzzy_match) => (fuzzy_match.score(), fuzzy_match.matches().clone()),
        _ => return Ok(None),
      },
      None => (0, vec![]),
    };

    let url_highlights = vec![];
    let tags_highlights = vec![];

    let secret_type = match entry.get_type()? {
      index::SecretType::Login => SecretType::Login,
      index::SecretType::Licence => SecretType::Licence,
      index::SecretType::Wlan => SecretType::Wlan,
      index::SecretType::Note => SecretType::Note,
      index::SecretType::Password => SecretType::Password,
      index::SecretType::Other => SecretType::Other,
    };
    if !filter.secret_type.iter().all(|filter| filter == &secret_type) {
      return Ok(None)
    }

    Ok(Some(SecretEntryMatch {
      entry: SecretEntry {
        id: entry.get_id()?.to_string(),
        timestamp: Utc.timestamp_millis(entry.get_timestamp()),
        name: entry.get_name()?.to_zeroing(),
        secret_type,
        tags: Self::read_text_list(entry.get_tags()?)?,
        urls: Self::read_text_list(entry.get_urls()?)?,
        deleted: entry.get_deleted(),
      },
      name_score,
      name_highlights,
      url_highlights,
      tags_highlights,
    }))
  }

  fn set_text_list<I, S>(mut text_list: text_list::Builder, texts: I) -> SecretStoreResult<()>
  where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
  {
    for (idx, text) in texts.into_iter().enumerate() {
      text_list.set(idx as u32, capnp::text::new_reader(text.as_ref().as_bytes())?);
    }
    Ok(())
  }

  fn read_text_list(text_list: text_list::Reader) -> SecretStoreResult<Vec<ZeroingString>> {
    let mut result = Vec::with_capacity(text_list.len() as usize);

    for maybe_text in text_list {
      result.push(maybe_text?.to_zeroing())
    }
    Ok(result)
  }
}

impl Default for Index {
  fn default() -> Self {
    let mut index_message = message::Builder::new(ZeroingHHeapAllocator::new());
    index_message.init_root::<index::Builder>();
    let index_data = serialize::write_message_to_words(&index_message);

    Index {
      data: index_data.into(),
      heads: HashMap::new(),
    }
  }
}
