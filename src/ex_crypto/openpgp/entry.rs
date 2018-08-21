use super::packet::PublicKey;

#[derive(Debug, PartialEq, Eq)]
pub struct Entry {
    primary_key: PublicKey,
    user_id: String,
}

