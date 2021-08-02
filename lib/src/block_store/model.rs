use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
  Add,
  Delete,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Change {
  pub op: Operation,
  pub block: String,
}

impl Change {
  pub fn new<T: Into<String>>(op: Operation, block: T) -> Change {
    Change {
      op,
      block: block.into(),
    }
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeLog {
  pub node: String,
  pub changes: Vec<Change>,
}

impl ChangeLog {
  pub fn new<T: Into<String>>(node: T) -> ChangeLog {
    ChangeLog {
      node: node.into(),
      changes: vec![],
    }
  }

  pub fn changes_since(&self, maybe_change: Option<&Change>) -> impl Iterator<Item = &Change> {
    let skip = maybe_change
      .and_then(|change| self.changes.iter().position(|c| c == change).map(|pos| pos + 1))
      .unwrap_or(0);

    self.changes.iter().dropping(skip)
  }
}
