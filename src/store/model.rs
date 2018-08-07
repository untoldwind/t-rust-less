use chrono::{DateTime, Utc};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Operation {
    Add,
    Delete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commit {
    node: String,
    prev: String,
    timestamp: DateTime<Utc>,
    changes: Vec<Change>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Head {
    node: String,
    commit: String,
}
