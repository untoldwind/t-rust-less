use chrono::{DateTime, Utc};

pub enum Operation {
    Add,
    Delete,
}

pub struct Change {
    op: Operation,
    block: String,
}

pub struct ChangeLog {
    pub node : String,
    pub changes : Vec<Change>,
}

pub struct Commit {
    node : String,
    prev: String,
    timestamp: DateTime<Utc>,
    changes: Vec<Change>,
}

pub struct Head {
    node: String,
    commit: String, 
}