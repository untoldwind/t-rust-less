use crate::api::{SecretEntry, SecretEntryMatch, SecretList, SecretListFilter, SecretVersion, SecretVersionRef};
use crate::api_capnp::secret_entry;
use crate::block_store::{Change, ChangeLog, Operation};
use crate::memguard::weak::{ZeroingHeapAllocator, ZeroingStringExt};
use crate::memguard::SecretWords;
use crate::secrets_store::{SecretStoreError, SecretStoreResult};
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

pub struct Index {
  heads: HashMap<String, Change>,
  pub(super) data: SecretWords,
}

impl Index {
  pub fn from_secured_raw(raw: &[u8]) -> SecretStoreResult<Index> {
    let data = SecretWords::from_secured(raw);
    let heads = Self::read_heads(&data)?;

    Ok(Index { data, heads })
  }

  pub fn find_versions(&self, secret_id: &str) -> SecretStoreResult<Vec<SecretVersionRef>> {
    let data_borrow = self.data.borrow();
    let reader = serialize::read_message_from_words(&data_borrow, message::ReaderOptions::new())?;
    let index = reader.get_root::<index::Reader>()?;

    for index_entry in index.get_entries()? {
      if index_entry.get_entry()?.get_id()? == secret_id {
        return Ok(
          index_entry
            .get_version_refs()?
            .iter()
            .map(SecretVersionRef::from_reader)
            .collect::<capnp::Result<Vec<SecretVersionRef>>>()?,
        );
      }
    }
    Err(SecretStoreError::NotFound)
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
    entries.sort();

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

        new_entries.set_with_caveats(entry_pos, old_index_entry)?;
        Self::update_entry(
          old_index_entry
            .get_version_refs()?
            .iter()
            .map(SecretVersionRef::from_reader)
            .collect::<capnp::Result<Vec<SecretVersionRef>>>()?,
          new_entries.reborrow().get(entry_pos),
          effective_changes.added_versions.get(secret_id),
          &effective_changes.deleted_blocks,
          &version_accessor,
        )?;
        entry_pos += 1;
      }
      for (secret_id, added_version) in effective_changes.added_versions {
        if to_keep.contains(&secret_id) {
          continue;
        }
        Self::update_entry(
          vec![],
          new_entries.reborrow().get(entry_pos),
          Some(&added_version),
          &effective_changes.deleted_blocks,
          &version_accessor,
        )?;
        entry_pos += 1;
      }
    }

    self.data = SecretWords::from(serialize::write_message_to_words(&index_message));
    self.heads = effective_changes.new_heads;

    Ok(true)
  }

  fn read_heads(index_data: &SecretWords) -> SecretStoreResult<HashMap<String, Change>> {
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
      for version_ref in index_entry.get_version_refs()? {
        let block_id = version_ref.get_block_id()?;
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
      let changes = change_log.changes_since(self.heads.get(&change_log.node));

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

  fn update_entry<F>(
    old_version_refs: Vec<SecretVersionRef>,
    mut new_entry: index::entry::Builder,
    maybe_added_versions: Option<&HashMap<String, SecretVersion>>,
    deleted_blocks: &HashSet<String>,
    version_accessor: F,
  ) -> SecretStoreResult<()>
  where
    F: Fn(&str) -> SecretStoreResult<Option<SecretVersion>>,
  {
    let current_block_id = old_version_refs.first().map(|v| v.block_id.clone());
    let mut version_refs =
      Vec::with_capacity(old_version_refs.len() + maybe_added_versions.map(HashMap::len).unwrap_or(0));

    for version_ref in old_version_refs {
      if !deleted_blocks.contains(&version_ref.block_id) {
        version_refs.push(version_ref)
      }
    }
    if let Some(added_versions) = maybe_added_versions {
      for (block_id, added_version) in added_versions {
        if !deleted_blocks.contains(block_id) {
          version_refs.push(SecretVersionRef {
            block_id: block_id.clone(),
            timestamp: added_version.timestamp,
          })
        }
      }
    }
    version_refs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    assert!(!version_refs.is_empty());

    let new_current_block_id = version_refs.first().unwrap().block_id.clone();
    if current_block_id.is_none() || current_block_id.unwrap() != new_current_block_id {
      match maybe_added_versions.and_then(|added| added.get(&new_current_block_id)) {
        Some(added_version) => added_version.to_entry_builder(new_entry.reborrow().init_entry())?,
        None => version_accessor(&new_current_block_id)?
          .unwrap()
          .to_entry_builder(new_entry.reborrow().init_entry())?,
      };
    }

    let mut entry_version_refs = new_entry.init_version_refs(version_refs.len() as u32);
    for (idx, version_ref) in version_refs.iter().enumerate() {
      version_ref.to_builder(entry_version_refs.reborrow().get(idx as u32));
    }

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
      heads: HashMap::new(),
    }
  }
}
