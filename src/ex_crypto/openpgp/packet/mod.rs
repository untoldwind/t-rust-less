mod tags;
mod ecc_curve;
mod keys;
mod symmetric;
mod parse;
mod sig;
mod hash;
mod util;

pub use self::tags::*;
pub use self::keys::PublicKey;

use chrono::{DateTime, Utc};
use self::symmetric::SymmetricKeyAlgorithm;
use self::keys::{public_key, private_key, PublicKeyAlgorithm, PrivateKey};
use self::sig::Signature;
use self::hash::HashAlgorithm;
use ex_crypto::error::Result;

#[cfg(test)]
mod tests;

/// Represents a Packet. A packet is the record structure used to encode a chunk of data in OpenPGP.
/// Ref: https://tools.ietf.org/html/rfc4880.html#section-4
#[derive(Debug, PartialEq, Eq)]
pub enum Packet {
    /// Public-Key Encrypted Session Key Packet
    PublicKeyEncryptedSessionKey(Version, Tag, Vec<u8>),
    /// Signature Packet
    Signature(Signature),
    /// Symmetric-Key Encrypted Session Key Packet
    SymKeyEncryptedSessionKey(Version, Tag, Vec<u8>),
    /// One-Pass Signature Packet
    OnePassSignature(Version, Tag, Vec<u8>),
    /// Secret-Key Packet
    SecretKey(PrivateKey),
    /// Public-Key Packet
    PublicKey(PublicKey),
    /// Secret-Subkey Packet
    SecretSubkey(PrivateKey),
    /// Compressed Data Packet
    CompressedData(Version, Tag, Vec<u8>),
    /// Symmetrically Encrypted Data Packet
    SymetricEncryptedData(Version, Tag, Vec<u8>),
    /// Marker Packet
    Marker(Version, Tag, Vec<u8>),
    /// Literal Data Packet
    Literal(Version, Tag, Vec<u8>),
    /// Trust Packet
    Trust(Version, Tag, Vec<u8>),
    /// User ID Packet
    UserID(String),
    /// Public-Subkey Packet
    PublicSubkey(PublicKey),
    /// User Attribute Packet
    UserAttribute(Version, Tag, Vec<u8>),
    /// Sym. Encrypted and Integrity Protected Data Packet
    SymEncryptedProtectedData(Version, Tag, Vec<u8>),
    /// Modification Detection Code Packet
    ModDetectionCode(Version, Tag, Vec<u8>),
}

impl Packet {
    pub fn new(version: Version, tag: Tag, body: Vec<u8>) -> Result<Packet> {
        match tag {
            Tag::PublicKeyEncryptedSessionKey => Ok(Packet::PublicKeyEncryptedSessionKey(version, tag, body)),
            Tag::Signature => Ok(Packet::Signature(sig::parser(&body)?.1)),
            Tag::SymKeyEncryptedSessionKey => Ok(Packet::SymKeyEncryptedSessionKey(version, tag, body)),
            Tag::OnePassSignature => Ok(Packet::OnePassSignature(version, tag, body)),
            Tag::SecretKey => Ok(Packet::SecretKey(private_key::parser(&body)?.1)),
            Tag::PublicKey => Ok(Packet::PublicKey(public_key::parser(&body)?.1)),
            Tag::SecretSubkey => Ok(Packet::SecretSubkey(private_key::parser(&body)?.1)),
            Tag::CompressedData => Ok(Packet::CompressedData(version, tag, body)),
            Tag::SymetricEncryptedData => Ok(Packet::SymetricEncryptedData(version, tag, body)),
            Tag::Marker => Ok(Packet::Marker(version, tag, body)),
            Tag::Literal => Ok(Packet::Literal(version, tag, body)),
            Tag::Trust => Ok(Packet::Trust(version, tag, body)),
            Tag::UserID => Ok(Packet::UserID(String::from_utf8_lossy(&body).into())),
            Tag::PublicSubkey => Ok(Packet::PublicSubkey(public_key::parser(&body)?.1)),
            Tag::UserAttribute => Ok(Packet::UserAttribute(version, tag, body)),
            Tag::SymEncryptedProtectedData => Ok(Packet::SymEncryptedProtectedData(version, tag, body)),
            Tag::ModDetectionCode => Ok(Packet::ModDetectionCode(version, tag, body)),
        }
    }
}
