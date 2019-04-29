use crate::api::{set_text_list, SecretEntry, SecretEntryMatch, SecretList, SecretListFilter, SecretVersion};
use crate::api_capnp::secret_entry;
use crate::block_store::{Change, ChangeLog, Operation};
use crate::memguard::weak::{ZeroingHeapAllocator, ZeroingStringExt};
use crate::memguard::SecretWords;
use crate::secrets_store::SecretStoreResult;
use crate::secrets_store_capnp::index;
use capnp::{message, serialize};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

struct EffectiveChanges {
  new_heads: HashMap<String, Change>,
  added_versions: HashMap<String, HashMap<String, SecretVersion>>,
  deleted_blocks: HashSet<String>,
}

impl EffectiveChanges {
  fn is_empty(&self) -> bool {
    self.added_versions.is_empty() && self.deleted_blocks.is_empty()
  }
}

pub struct BlockIndex {
  heads: HashMap<String, Change>,
  pub(super) current_blocks: HashMap<String, (String, bool)>,
}

pub struct Index {
  pub(super) data: SecretWords,
  pub(super) block_index: BlockIndex,
}

impl Index {
  pub fn from_secured_raw(raw: &[u8]) -> SecretStoreResult<Index> {
    let data = SecretWords::from_secured(raw);
    let block_index = Self::read_block_index(&data)?;

    Ok(Index { data, block_index })
  }

  pub fn filter_entries(&self, filter: SecretListFilter) -> SecretStoreResult<SecretList> {
    let data_borrow = self.data.borrow();
    let reader = serialize::read_message_from_words(&data_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut entries = Vec::new();
    let mut all_tags = HashSet::new();

    for index_entry in index.get_entries()? {
      let entry = index_entry.get_entry()?;
      for maybe_tag in entry.get_tags()? {
        let tag = maybe_tag?;
        if !all_tags.contains(tag) {
          all_tags.insert(tag.to_zeroing());
        }
      }
      if let Some(entry_match) = Self::match_entry(entry, &filter)? {
        entries.push(entry_match);
      }
    }

    Ok(SecretList {
      all_tags: all_tags.into_iter().collect(),
      entries,
    })
  }

  pub fn process_change_logs<F>(&mut self, change_logs: &[ChangeLog], version_accessor: F) -> SecretStoreResult<bool>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let effective_changes = self.collect_changes(change_logs, &version_accessor)?;

    if effective_changes.is_empty() {
      return Ok(false); // No change that affects us
    }

    let to_keep = self.collect_entries_to_keep(&effective_changes.deleted_blocks)?;
    let additions = effective_changes
      .added_versions
      .keys()
      .filter(|id| !to_keep.contains(*id))
      .count();
    let mut current_blocks = HashMap::new();
    let mut index_message = message::Builder::new(ZeroingHeapAllocator::default());
    {
      let index_borrow = self.data.borrow();
      let reader = serialize::read_message_from_words(&index_borrow, message::ReaderOptions::new())?;
      let old_index = reader.get_root::<index::Reader>()?;
      let mut new_index = index_message.init_root::<index::Builder>();

      Self::update_heads(new_index.reborrow(), &effective_changes.new_heads);
      let mut entry_pos = 0;
      let mut new_entries = new_index.init_entries((to_keep.len() + additions) as u32);

      for old_index_entry in old_index.get_entries()? {
        let old_entry = old_index_entry.get_entry()?;
        let secret_id = old_entry.get_id()?;
        if !to_keep.contains(secret_id) {
          continue;
        }
        let new_block_ids = Self::merge_block_ids(
          old_index_entry.reborrow(),
          effective_changes.added_versions.get(secret_id).map(HashMap::keys),
          &effective_changes.deleted_blocks,
        )?;

        if effective_changes
          .deleted_blocks
          .contains(old_index_entry.get_current_block_id()?)
        {
          let (current_block_id, has_versions) = Self::recreate_entry(
            new_entries.reborrow().get(entry_pos),
            new_block_ids,
            effective_changes.added_versions.get(secret_id),
            &version_accessor,
          )?;
          entry_pos += 1;
          current_blocks.insert(secret_id.to_string(), (current_block_id, has_versions));
        } else {
          new_entries.set_with_caveats(entry_pos, old_index_entry)?;
          let (current_block_id, has_versions) = Self::update_entry(
            new_entries.reborrow().get(entry_pos),
            new_block_ids,
            effective_changes.added_versions.get(secret_id),
          )?;
          entry_pos += 1;
          current_blocks.insert(secret_id.to_string(), (current_block_id, has_versions));
        }
      }
      for (secret_id, added_version) in effective_changes.added_versions {
        if to_keep.contains(&secret_id) {
          continue;
        }
        let (current_block_id, has_versions) = Self::update_entry(
          new_entries.reborrow().get(entry_pos),
          added_version.keys().cloned().collect(),
          Some(&added_version),
        )?;
        entry_pos += 1;
        current_blocks.insert(secret_id.to_string(), (current_block_id, has_versions));
      }
    }

    self.data = SecretWords::from(serialize::write_message_to_words(&index_message));
    self.block_index.heads = effective_changes.new_heads;
    self.block_index.current_blocks = current_blocks;

    Ok(true)
  }

  fn read_block_index(index_data: &SecretWords) -> SecretStoreResult<BlockIndex> {
    let index_borrow = index_data.borrow();
    let reader = serialize::read_message_from_words(&index_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut heads = HashMap::with_capacity(index.get_heads()?.len() as usize);
    let mut current_blocks = HashMap::with_capacity(index.get_entries()?.len() as usize);

    for head in index.get_heads()? {
      let node_id = head.get_node_id()?.to_string();
      let op = match head.get_operation()? {
        index::HeadOperation::Add => Operation::Add,
        index::HeadOperation::Delete => Operation::Delete,
      };
      let block = head.get_block_id()?.to_string();
      heads.insert(node_id, Change { op, block });
    }
    for index_entry in index.get_entries()? {
      let entry = index_entry.get_entry()?;
      let secret_id = entry.get_id()?;
      let current_block_id = index_entry.get_current_block_id()?;
      let hash_versions = index_entry.get_block_ids()?.len() > 1;

      current_blocks.insert(secret_id.to_string(), (current_block_id.to_string(), hash_versions));
    }

    Ok(BlockIndex { heads, current_blocks })
  }

  fn update_heads(index: index::Builder, heads: &HashMap<String, Change>) {
    let mut new_heads = index.init_heads(heads.len() as u32);

    for (idx, (node_id, change)) in heads.iter().enumerate() {
      let mut new_head = new_heads.reborrow().get(idx as u32);

      new_head.set_node_id(&node_id);
      match change.op {
        Operation::Add => new_head.set_operation(index::HeadOperation::Add),
        Operation::Delete => new_head.set_operation(index::HeadOperation::Delete),
      }
      new_head.set_block_id(&change.block);
    }
  }

  fn collect_entries_to_keep(&self, deleted_blocks: &HashSet<String>) -> SecretStoreResult<HashSet<String>> {
    let data_borrow = self.data.borrow();
    let reader = serialize::read_message_from_words(&data_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;
    let mut to_keep = HashSet::new();

    for index_entry in index.get_entries()? {
      let entry = index_entry.get_entry()?;
      let secret_id = entry.get_id()?;
      let mut remainging_count = 0;
      for maybe_block_id in index_entry.get_block_ids()? {
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
  ) -> SecretStoreResult<EffectiveChanges>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let mut new_heads = HashMap::with_capacity(change_logs.len());
    let mut added_versions = HashMap::new();
    let mut deleted_blocks = HashSet::new();

    for change_log in change_logs {
      let changes = change_log.changes_since(self.block_index.heads.get(&change_log.node));

      for change in changes {
        match change.op {
          Operation::Add => {
            if let Some(secret_version) = version_accessor(&change.block)? {
              let secret_id = secret_version.secret_id.clone();
              let mut by_blocks = added_versions.remove(&secret_id).unwrap_or_else(HashMap::new);
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

    Ok(EffectiveChanges {
      new_heads,
      added_versions,
      deleted_blocks,
    })
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
  ) -> SecretStoreResult<(String, bool)>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let mut current_block_id = new_entry.reborrow().get_current_block_id()?.to_string();

    for block_id in new_block_ids.clone() {
      let maybe_version = match maybe_added_versions.and_then(|a| a.get(&block_id)) {
        None => version_accessor(&block_id)?,
        Some(version) => Some(version.clone()),
      };
      match maybe_version {
        Some(version) => {
          if new_entry.reborrow().get_current_block_id()?.is_empty()
            || new_entry.reborrow().get_entry()?.get_timestamp() < version.timestamp.timestamp_millis()
          {
            Self::overwrite_entry(new_entry.reborrow(), &block_id, &version)?;
            current_block_id = block_id.clone();
          }
        }
        None => {
          new_block_ids.remove(&block_id);
        }
      }
    }

    set_text_list(
      new_entry.reborrow().init_block_ids(new_block_ids.len() as u32),
      &new_block_ids,
    )?;

    Ok((current_block_id, new_block_ids.len() > 1))
  }

  fn update_entry(
    mut new_entry: index::entry::Builder,
    mut new_block_ids: HashSet<String>,
    maybe_added_versions: Option<&HashMap<String, SecretVersion>>,
  ) -> SecretStoreResult<(String, bool)> {
    let mut current_block_id = new_entry.reborrow().get_current_block_id()?.to_string();

    for block_id in new_block_ids.clone() {
      let maybe_version = maybe_added_versions.and_then(|a| a.get(&block_id));
      match maybe_version {
        Some(version) => {
          if new_entry.reborrow().get_current_block_id()?.is_empty()
            || new_entry.reborrow().get_entry()?.get_timestamp() < version.timestamp.timestamp_millis()
          {
            Self::overwrite_entry(new_entry.reborrow(), &block_id, version)?;
            current_block_id = block_id.clone();
          }
        }
        None => {
          new_block_ids.remove(&block_id);
        }
      }
    }

    set_text_list(
      new_entry.reborrow().init_block_ids(new_block_ids.len() as u32),
      &new_block_ids,
    )?;

    Ok((current_block_id, new_block_ids.len() > 1))
  }

  fn overwrite_entry(
    mut new_index_entry: index::entry::Builder,
    block_id: &str,
    version: &SecretVersion,
  ) -> SecretStoreResult<()> {
    let mut new_entry = new_index_entry.reborrow().get_entry()?;
    new_entry.set_id(&version.secret_id);
    new_entry.set_timestamp(version.timestamp.timestamp_millis());
    new_entry.set_name(&version.name);
    new_entry.set_type(version.secret_type.to_builder());
    set_text_list(new_entry.reborrow().init_tags(version.tags.len() as u32), &version.tags)?;
    set_text_list(new_entry.reborrow().init_urls(version.urls.len() as u32), &version.urls)?;
    new_entry.set_deleted(version.deleted);
    new_index_entry.set_current_block_id(block_id);

    Ok(())
  }

  fn match_entry(
    entry_reader: secret_entry::Reader,
    filter: &SecretListFilter,
  ) -> SecretStoreResult<Option<SecretEntryMatch>> {
    let entry = SecretEntry::from_reader(entry_reader)?;
    if filter.deleted != entry.deleted {
      return Ok(None);
    }

    let (name_score, name_highlights) = match &filter.name {
      Some(name_filter) => match sublime_fuzzy::best_match(name_filter, &entry.name) {
        Some(fuzzy_match) => (fuzzy_match.score(), fuzzy_match.matches().clone()),
        _ => return Ok(None),
      },
      None => (0, vec![]),
    };

    let url_highlights = vec![];
    let tags_highlights = match &filter.tag {
      Some(tag_filter) => {
        let highlights: Vec<usize> = entry
          .tags
          .iter()
          .positions(|tag| tag.as_str() == tag_filter.as_str())
          .collect();
        if highlights.is_empty() {
          return Ok(None);
        }
        highlights
      }
      None => vec![],
    };

    if !filter.secret_type.iter().all(|filter| filter == &entry.secret_type) {
      return Ok(None);
    }

    Ok(Some(SecretEntryMatch {
      entry,
      name_score,
      name_highlights,
      url_highlights,
      tags_highlights,
    }))
  }
}

impl Default for Index {
  fn default() -> Self {
    let mut index_message = message::Builder::new(ZeroingHeapAllocator::default());
    index_message.init_root::<index::Builder>();
    let index_data = serialize::write_message_to_words(&index_message);

    Index {
      data: index_data.into(),
      block_index: BlockIndex {
        heads: HashMap::new(),
        current_blocks: HashMap::new(),
      },
    }
  }
}
