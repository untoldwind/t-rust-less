use super::ecc_curve::{ecc_curve_from_oid, ECCCurve};
use super::symmetric::SymmetricKeyAlgorithm;
use openssl::bn::BigNum;
use nom::{self, be_u16, be_u32, be_u8};
use super::util::{bignum_to_mpi, mpi_big};
use num_traits::FromPrimitive;
use openssl::rsa::{Rsa, RsaPrivateKeyBuilder};
use ex_crypto::error::Result;
use openssl::pkey;
use openssl::dsa::Dsa;
use byteorder::{BigEndian, ByteOrder};
use openssl::hash::{Hasher, MessageDigest};

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive, ToPrimitive)]
pub enum PublicKeyAlgorithm {
    /// RSA (Encrypt and Sign) [HAC]
    RSA = 1,
    /// DEPRECATED: RSA (Encrypt-Only) [HAC]
    RSAEncrypt = 2,
    /// DEPRECATED: RSA (Sign-Only) [HAC]
    RSASign = 3,
    /// Elgamal (Encrypt-Only) [ELGAMAL] [HAC]
    ElgamalSign = 16,
    /// DSA (Digital Signature Algorithm) [FIPS186] [HAC]
    DSA = 17,
    /// Elliptic Curve: RFC-6637
    ECDH = 18,
    /// ECDSA: RFC-6637
    ECDSA = 19,
    /// DEPRECATED: Elgamal (Encrypt and Sign)
    Elgamal = 20,
    /// Reserved for Diffie-Hellman (X9.42, as defined for IETF-S/MIME)
    DiffieHellman = 21,
    /// EdDSA (not yet assigned)
    EdDSA = 22,
    /// Private experimental range (from OpenGPG)
    // TODO: genenric Unknown(u8)
    Private100 = 100,
    Private101 = 101,
    Private102 = 102,
    Private103 = 103,
    Private104 = 104,
    Private105 = 105,
    Private106 = 106,
    Private107 = 107,
    Private108 = 108,
    Private109 = 109,
    Private110 = 110,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromPrimitive, ToPrimitive)]
pub enum KeyVersion {
    V2 = 2,
    V3 = 3,
    V4 = 4,
}

#[derive(Debug, PartialEq, Eq, Clone, FromPrimitive, ToPrimitive)]
/// Available String-To-Key types
pub enum StringToKeyType {
    Simple = 0,
    Salted = 1,
    Reserved = 2,
    IteratedAndSalted = 3,
    Private100 = 100,
    Private101 = 101,
    Private102 = 102,
    Private103 = 103,
    Private104 = 104,
    Private105 = 105,
    Private106 = 106,
    Private107 = 107,
    Private108 = 108,
    Private109 = 109,
    Private110 = 110,
}

impl StringToKeyType {
    pub fn param_len(&self) -> usize {
        match self {
            // 1 octet hash algorithm.
            StringToKeyType::Simple => 1,
            // Salted has 1 octet hash algorithm and 8 octets salt value.
            StringToKeyType::Salted => 9,
            // Salted and iterated has 1 octet hash algorithm, 8 octets salt value and 1 octet count.
            StringToKeyType::IteratedAndSalted => 10,
            _ => 0,
        }
    }
}

/// Represents a single private key packet.
#[derive(Debug, PartialEq, Eq)]
pub struct PrivateKey {
    version: KeyVersion,
    algorithm: PublicKeyAlgorithm,
    created_at: u32,
    expiration: Option<u16>,
    public_params: PublicParams,
    private_params: EncryptedPrivateParams,
}

impl PrivateKey {
    pub fn new(
        version: KeyVersion,
        algorithm: PublicKeyAlgorithm,
        created_at: u32,
        expiration: Option<u16>,
        public_params: PublicParams,
        private_params: EncryptedPrivateParams,
    ) -> PrivateKey {
        PrivateKey {
            version,
            algorithm,
            created_at,
            expiration,
            public_params,
            private_params,
        }
    }

    /// Unlock the raw data in the secret parameters.
    pub fn unlock<'a>(&self, pw: fn() -> &'a str, work: fn(&PrivateKeyRepr) -> Result<()>) -> Result<()> {
        let decrypted = if self.private_params.is_encrypted() {
            self.from_ciphertext(pw, self.private_params.data.as_slice())
        } else {
            self.from_plaintext(self.private_params.data.as_slice())
        }?;

        work(&decrypted)
    }

    fn from_ciphertext<'a>(&self, _pw: fn() -> &'a str, _ciphertext: &[u8]) -> Result<PrivateKeyRepr> {
        match self.algorithm {
            PublicKeyAlgorithm::RSA | PublicKeyAlgorithm::RSAEncrypt | PublicKeyAlgorithm::RSASign => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::DSA => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::ECDH => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::ECDSA => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::EdDSA => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::Elgamal => {
                unimplemented!("implement me");
            }
            _ => panic!("unsupported algoritm: {:?}", self.algorithm),
        }
    }

    fn from_plaintext(&self, plaintext: &[u8]) -> Result<PrivateKeyRepr> {
        match self.algorithm {
            PublicKeyAlgorithm::RSA | PublicKeyAlgorithm::RSAEncrypt | PublicKeyAlgorithm::RSASign => {
                let (_, (d, p, q, u)) = private_key::rsa_private_params(plaintext)?;
                match self.public_params {
                    PublicParams::RSA { ref n, ref e } => {
                        // create an actual openssl key
                        // Sad but true
                        let n = BigNum::from_slice(n.to_vec().as_slice())?;
                        let e = BigNum::from_slice(e.to_vec().as_slice())?;
                        let private_key = RsaPrivateKeyBuilder::new(n, e, d)?
                            .set_factors(p, q)?
                            .build();
                        println!("got a private key :) {:?}", private_key);

                        Ok(PrivateKeyRepr::RSA(private_key))
                    }
                    _ => unreachable!("inconsistent key state"),
                }
            }
            PublicKeyAlgorithm::DSA => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::ECDH => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::ECDSA => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::EdDSA => {
                unimplemented!("implement me");
            }
            PublicKeyAlgorithm::Elgamal => {
                unimplemented!("implement me");
            }
            _ => panic!("unsupported algoritm: {:?}", self.algorithm),
        }
    }

    pub fn private_params(&self) -> &EncryptedPrivateParams {
        &self.private_params
    }
}

/// Represents a single public key packet.
#[derive(Debug, PartialEq, Eq)]
pub struct PublicKey {
    version: KeyVersion,
    algorithm: PublicKeyAlgorithm,
    created_at: u32,
    expiration: Option<u16>,
    public_params: PublicParams,
}

impl PublicKey {
    pub fn new(
        version: KeyVersion,
        algorithm: PublicKeyAlgorithm,
        created_at: u32,
        expiration: Option<u16>,
        public_params: PublicParams,
    ) -> PublicKey {
        PublicKey {
            version,
            algorithm,
            created_at,
            expiration,
            public_params,
        }
    }
}

/// Represent the public paramaters for the different algorithms.
#[derive(Debug, PartialEq, Eq)]
pub enum PublicParams {
    RSA {
        n: BigNum,
        e: BigNum,
    },
    DSA {
        p: BigNum,
        q: BigNum,
        g: BigNum,
        y: BigNum,
    },
    ECDSA {
        curve: ECCCurve,
        p: BigNum,
    },
    ECDH {
        curve: ECCCurve,
        p: BigNum,
        hash: u8,
        alg_sym: u8,
    },
    Elgamal {
        p: BigNum,
        g: BigNum,
        y: BigNum,
    },
}

/// this is the version of the private key that is actually exposed to users to
/// do crypto operations.
#[derive(Debug)]
pub enum PrivateKeyRepr {
    RSA(Rsa<pkey::Private>),
    DSA(Dsa<pkey::Private>),
}

/// A list of params that are used to represent the values of possibly encrypted key, from imports and exports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncryptedPrivateParams {
    /// The raw data as generated when imported.
    pub data: Vec<u8>,
    /// Hash or checksum of the raw data.
    pub checksum: Vec<u8>,
    /// IV, exist encrypted raw data.
    pub iv: Option<Vec<u8>>,
    /// If raw is encrypted, the encryption algorithm used.
    pub encryption_algorithm: Option<SymmetricKeyAlgorithm>,
    /// If raw is encrypted, the string-to-key method used.
    pub string_to_key: Option<StringToKeyType>,
    /// If raw is encrypted, the params for the string-to-key method.
    pub string_to_key_params: Option<Vec<u8>>,
    /// The identifier for how this data is stored.
    pub string_to_key_id: u8,
}

impl EncryptedPrivateParams {
    pub fn new_plaintext(data: Vec<u8>, checksum: Vec<u8>) -> EncryptedPrivateParams {
        EncryptedPrivateParams {
            data,
            checksum,
            iv: None,
            encryption_algorithm: None,
            string_to_key: None,
            string_to_key_id: 0,
            string_to_key_params: None,
        }
    }

    pub fn is_encrypted(&self) -> bool {
        self.string_to_key_id != 0
    }
}

pub mod public_key {
    use super::{KeyVersion, PublicKey, PublicKeyAlgorithm, PublicParams};
    use nom::{self, be_u16, be_u32, be_u8};
    use ex_crypto::openpgp::packet::ecc_curve::{ecc_curve_from_oid, ECCCurve};
    use ex_crypto::openpgp::packet::util::mpi_big;
    use num_traits::FromPrimitive;

    // Ref: https://tools.ietf.org/html/rfc6637#section-9
    named!(
        ecdsa<PublicParams>,
        do_parse!(
            // a one-octet size of the following field
            len: be_u8
    // octets representing a curve OID
    >> curve: map_opt!(take!(len), ecc_curve_from_oid)
    // MPI of an EC point representing a public key
    >>   p: mpi_big >> (PublicParams::ECDSA { curve, p })
        )
    );

    // Ref: https://tools.ietf.org/html/rfc6637#section-9
    named!(
        ecdh<PublicParams>,
        do_parse!(
            // a one-octet size of the following field
            len: be_u8
    // octets representing a curve OID
    >>  curve: map_opt!(take!(len), ecc_curve_from_oid)
    // MPI of an EC point representing a public key
    >>    p: mpi_big
    // a one-octet size of the following fields
    >> _len2: be_u8
    // a one-octet value 01, reserved for future extensions
    >>       tag!(&[1][..])
    // a one-octet hash function ID used with a KDF
    >> hash: take!(1)
    // a one-octet algorithm ID for the symmetric algorithm used to wrap
    // the symmetric key used for the message encryption
    >>  alg_sym: take!(1) >> (PublicParams::ECDH {
                curve,
                p,
                hash: hash[0],
                alg_sym: alg_sym[0],
            })
        )
    );

    named!(
        elgamal<PublicParams>,
        do_parse!(
            // MPI of Elgamal prime p
            p: mpi_big
    // MPI of Elgamal group generator g
    >> g: mpi_big
    // MPI of Elgamal public key value y (= g**x mod p where x is secret)
    >> y: mpi_big >> (PublicParams::Elgamal { p, g, y })
        )
    );

    named!(
        dsa<PublicParams>,
        do_parse!(p: mpi_big >> q: mpi_big >> g: mpi_big >> y: mpi_big >> (PublicParams::DSA { p, q, g, y }))
    );

    named!(
        rsa<PublicParams>,
        do_parse!(n: mpi_big >> e: mpi_big >> (PublicParams::RSA { n, e }))
    );

    named_args!(key_from_fields<'a>(typ: &PublicKeyAlgorithm) <PublicParams>, switch!(
        value!(typ),
        &PublicKeyAlgorithm::RSA        |
        &PublicKeyAlgorithm::RSAEncrypt |
        &PublicKeyAlgorithm::RSASign    => call!(rsa)     |
        &PublicKeyAlgorithm::DSA        => call!(dsa)     |
        &PublicKeyAlgorithm::ECDSA      => call!(ecdsa)   |
        &PublicKeyAlgorithm::ECDH       => call!(ecdh)    |
        &PublicKeyAlgorithm::Elgamal    |
        &PublicKeyAlgorithm::ElgamalSign => call!(elgamal)
        // &PublicKeyAlgorithm::DiffieHellman =>
    ));

    named_args!(new_public_key_parser<'a>(key_ver: &'a KeyVersion) <PublicKey>, do_parse!(
           created_at: be_u32
        >>        alg: map_opt!(be_u8, |v| PublicKeyAlgorithm::from_u8(v))
        >>     params: call!(key_from_fields, &alg)
        >> (PublicKey::new(*key_ver, alg, created_at, None, params))
    ));

    named_args!(old_public_key_parser<'a>(key_ver: &'a KeyVersion) <PublicKey>, do_parse!(
            created_at: be_u32
        >>         exp: be_u16
        >>         alg: map_opt!(be_u8, PublicKeyAlgorithm::from_u8)
        >>      params: call!(key_from_fields, &alg)
        >> (PublicKey::new(*key_ver, alg, created_at, Some(exp), params))
    ));

    /// Parse a public key packet (Tag 6)
    /// Ref: https://tools.ietf.org/html/rfc4880.html#section-5.5.1.1
    named!(pub parser<PublicKey>, do_parse!(
              key_ver: map_opt!(be_u8, KeyVersion::from_u8)
        >>    key: switch!(value!(&key_ver),
                           &KeyVersion::V2 => call!(old_public_key_parser, &key_ver) |
                           &KeyVersion::V3 => call!(old_public_key_parser, &key_ver) |
                           &KeyVersion::V4 => call!(new_public_key_parser, &key_ver)
                       )
        >> (key)
    ));
}

pub mod private_key {
    use super::{EncryptedPrivateParams, KeyVersion, PrivateKey, PublicKey, PublicKeyAlgorithm, PublicParams,
                StringToKeyType};
    use nom::{self, be_u16, be_u32, be_u8};
    use ex_crypto::openpgp::packet::ecc_curve::{ecc_curve_from_oid, ECCCurve};
    use ex_crypto::openpgp::packet::symmetric::SymmetricKeyAlgorithm;
    use ex_crypto::openpgp::packet::util::{mpi_big, rest_len};
    use num_traits::FromPrimitive;
    use openssl::bn::BigNum;

    // Ref: https://tools.ietf.org/html/rfc6637#section-9
    #[cfg_attr(rustfmt, rustfmt_skip)]
    named!(
        ecdsa<(PublicParams, EncryptedPrivateParams)>,
        do_parse!(
        // a one-octet size of the following field
           len: be_u8
        // octets representing a curve OID
        >> curve: map_opt!(take!(len), ecc_curve_from_oid)
        // MPI of an EC point representing a public key
        >>   p: mpi_big
        >> (PublicParams::ECDSA { curve, p }, EncryptedPrivateParams::new_plaintext(vec![], vec![]))
    ));

    // Ref: https://tools.ietf.org/html/rfc6637#section-9
    #[cfg_attr(rustfmt, rustfmt_skip)]
    named!(
        ecdh<(PublicParams, EncryptedPrivateParams)>,
        do_parse!(
        // a one-octet size of the following field
            len: be_u8
        // octets representing a curve OID
        >>  curve: map_opt!(take!(len), ecc_curve_from_oid)
        // MPI of an EC point representing a public key
        >>    p: mpi_big
        // a one-octet size of the following fields
        >> _len2: be_u8
        // a one-octet value 01, reserved for future extensions
        >>       tag!(&[1][..])
        // a one-octet hash function ID used with a KDF
        >> hash: take!(1)
        // a one-octet algorithm ID for the symmetric algorithm used to wrap
        // the symmetric key used for the message encryption
        >>  alg_sym: take!(1)
                >> (
                    PublicParams::ECDH {
            curve,
            p,
            hash: hash[0],
            alg_sym: alg_sym[0]
        }, EncryptedPrivateParams::new_plaintext(vec![], vec![]))
    ));

    #[cfg_attr(rustfmt, rustfmt_skip)]
    named!(
        elgamal<(PublicParams, EncryptedPrivateParams)>,
        do_parse!(
        // MPI of Elgamal prime p
           p: mpi_big
        // MPI of Elgamal group generator g
        >> g: mpi_big
        // MPI of Elgamal public key value y (= g**x mod p where x is secret)
        >> y: mpi_big
        >> (PublicParams::Elgamal {
                p,
                g,
                y,
            },
            EncryptedPrivateParams::new_plaintext(vec![], vec![]))
    ));

    #[cfg_attr(rustfmt, rustfmt_skip)]
    named!(dsa<(PublicParams, EncryptedPrivateParams)>, do_parse!(
           p: mpi_big
        >> q: mpi_big
        >> g: mpi_big
        >> y: mpi_big
        >> (PublicParams::DSA {
                p,
                q,
                g,
                y,
            },
            EncryptedPrivateParams::new_plaintext(vec![], vec![]))
    ));

    #[cfg_attr(rustfmt, rustfmt_skip)]
    named!(
        rsa<(PublicParams, EncryptedPrivateParams)>,
        do_parse!(
            n: mpi_big >> e: mpi_big >> s2k_typ: be_u8
                >> enc_params:
                    switch!(value!(s2k_typ),
            // 0 is no encryption
            0       => value!((None, None, None, None)) |
            // symmetric key algorithm
            1...253 => do_parse!(
                   sym_alg: map_opt!(value!(s2k_typ), SymmetricKeyAlgorithm::from_u8)
                >>      iv: take!(sym_alg.block_size())
                >> (Some(sym_alg), Some(iv), None, None)
            ) |
            // symmetric key + string-to-key
            254...255 => do_parse!(
                          sym_alg: map_opt!(be_u8, SymmetricKeyAlgorithm::from_u8)
                    >>        s2k: map_opt!(be_u8, StringToKeyType::from_u8)
                    >> s2k_params: take!(s2k.param_len())
                    >>         iv: take!(sym_alg.block_size())
                    >> (Some(sym_alg), Some(iv), Some(s2k), Some(s2k_params))
            )
        )
                >> checksum_len:
                    switch!(value!(s2k_typ),
                         // 20 octect hash at the end
                         254 => value!(20) |
                         // 2 octet checksum at the end
                         _   => value!(2)
        ) >> data_len: map!(rest_len, |r| r - checksum_len) >> data: take!(data_len)
        >> checksum: take!(checksum_len)
                >> (
                    PublicParams::RSA { n, e },
            EncryptedPrivateParams {
                data: data.to_vec(),
                checksum: checksum.to_vec(),
                iv: enc_params.1.map(|iv| iv.to_vec()),
                encryption_algorithm: enc_params.0,
                string_to_key: enc_params.2,
                string_to_key_params: enc_params.3.map(|p| p.to_vec()),
                string_to_key_id: s2k_typ,
            })
    ));

    named_args!(key_from_fields<'a>(typ: &'a PublicKeyAlgorithm) <(PublicParams, EncryptedPrivateParams)>, switch!(
        value!(&typ),
        &PublicKeyAlgorithm::RSA        |
        &PublicKeyAlgorithm::RSAEncrypt |
        &PublicKeyAlgorithm::RSASign    => call!(rsa)     |
        &PublicKeyAlgorithm::DSA        => call!(dsa)     |
        &PublicKeyAlgorithm::ECDSA      => call!(ecdsa)   |
        &PublicKeyAlgorithm::ECDH       => call!(ecdh)    |
        &PublicKeyAlgorithm::Elgamal    |
        &PublicKeyAlgorithm::ElgamalSign => call!(elgamal)
        // &PublicKeyAlgorithm::DiffieHellman =>
    ));

    named_args!(new_private_key_parser<'a>(key_ver: &'a KeyVersion) <PrivateKey>, do_parse!(
            created_at: be_u32
        >>         alg: map_opt!(be_u8, |v| PublicKeyAlgorithm::from_u8(v))
        >>      params: call!(key_from_fields, &alg)
        >> (PrivateKey::new(*key_ver, alg, created_at, None, params.0, params.1))
    ));

    named_args!(old_private_key_parser<'a>(key_ver: &'a KeyVersion) <PrivateKey>, do_parse!(
           created_at: be_u32
        >>        exp: be_u16
        >>        alg: map_opt!(be_u8, PublicKeyAlgorithm::from_u8)
        >>     params: call!(key_from_fields, &alg)
        >> (PrivateKey::new(*key_ver, alg, created_at, Some(exp), params.0, params.1))
    ));

    /// Parse a private key packet (Tag 5)
    /// Ref: https://tpools.ietf.org/html/rfc4880.html#section-5.5.1.3
    named!(pub parser<PrivateKey>, do_parse!(
              key_ver: map_opt!(be_u8, KeyVersion::from_u8)
        >>    key: switch!(value!(&key_ver),
                           &KeyVersion::V2 => call!(old_private_key_parser, &key_ver) |
                           &KeyVersion::V3 => call!(old_private_key_parser, &key_ver) |
                           &KeyVersion::V4 => call!(new_private_key_parser, &key_ver)
                       )
        >> (key)
    ));

    /// Parse the decrpyted private params of an RSA private key.
    named!(pub rsa_private_params<(BigNum, BigNum,BigNum, BigNum)>, do_parse!(
           d: mpi_big
        >> p: mpi_big
        >> q: mpi_big
        >> u: mpi_big
        >> (d, p, q, u)
    ));
}

macro_rules! key {
    ($name:ident) => {
        impl $name {
            pub fn version(&self) -> &KeyVersion {
                &self.version
            }

            pub fn algorithm(&self) -> &PublicKeyAlgorithm {
                &self.algorithm
            }

            pub fn created_at(&self) -> u32 {
                self.created_at
            }

            pub fn expiration(&self) -> Option<u16> {
                self.expiration
            }

            pub fn public_params(&self) -> &PublicParams {
                &self.public_params
            }

            /// Returns the fingerprint of this key.
            pub fn fingerprint(&self) -> Vec<u8> {
                match self.version() {
                    KeyVersion::V4 => {
                        // A one-octet version number (4).
                        let mut packet = Vec::new();
                        packet.push(4);

                        // A four-octet number denoting the time that the key was created.
                        let mut time_buf: [u8; 4] = [0; 4];
                        BigEndian::write_u32(&mut time_buf, self.created_at());
                        packet.extend_from_slice(&time_buf);

                        // A one-octet number denoting the public-key algorithm of this key.
                        packet.push(*self.algorithm() as u8);

                        // A series of multiprecision integers comprising the key material.
                        match &self.public_params {
                            PublicParams::RSA { n, e } => {
                                packet.extend(bignum_to_mpi(n));
                                packet.extend(bignum_to_mpi(e));
                            }
                            PublicParams::DSA { p, q, g, y } => {
                                packet.extend(bignum_to_mpi(p));
                                packet.extend(bignum_to_mpi(q));
                                packet.extend(bignum_to_mpi(g));
                                packet.extend(bignum_to_mpi(y));
                            }
                            PublicParams::ECDSA { curve, p } => {
                                //a one-octet size of the following field
                                packet.push(curve.oid().len() as u8);
                                //octets representing a curve OID
                                packet.extend(curve.oid().iter().cloned());
                                //MPI of an EC point representing a public key
                                packet.extend(bignum_to_mpi(p));
                            }
                            PublicParams::ECDH {
                                curve,
                                p,
                                hash,
                                alg_sym,
                            } => {
                                //a one-octet size of the following field
                                packet.push(curve.oid().len() as u8);
                                //the octets representing a curve OID
                                packet.extend(curve.oid().iter().cloned());
                                //MPI of an EC point representing a public key
                                packet.extend(bignum_to_mpi(p));
                                //a one-octet size of the following fields
                                packet.push(3); // Always 3??
                                //a one-octet value 01
                                packet.push(1);
                                //a one-octet hash function ID used with a KDF
                                packet.push(*hash);
                                //a one-octet algorithm ID
                                packet.push(*alg_sym);
                            }
                            PublicParams::Elgamal { p, g, y } => {
                                packet.extend(bignum_to_mpi(p));
                                packet.extend(bignum_to_mpi(g));
                                packet.extend(bignum_to_mpi(y));
                            }
                        }

                        let mut length_buf: [u8; 2] = [0; 2];
                        BigEndian::write_uint(&mut length_buf, packet.len() as u64, 2);

                        let mut h = Hasher::new(MessageDigest::sha1()).unwrap();

                        h.update(&[0x99]).unwrap();
                        h.update(&length_buf).unwrap();
                        h.update(&packet).unwrap();

                        h.finish().unwrap().to_vec()
                    }

                    KeyVersion::V2 | KeyVersion::V3 => {
                        let mut h = Hasher::new(MessageDigest::md5()).unwrap();

                        let mut packet = Vec::new();

                        match &self.public_params {
                            PublicParams::RSA { n, e } => {
                                packet.extend(bignum_to_mpi(n));
                                packet.extend(bignum_to_mpi(e));
                            }
                            PublicParams::DSA { p, q, g, y } => {
                                packet.extend(bignum_to_mpi(p));
                                packet.extend(bignum_to_mpi(q));
                                packet.extend(bignum_to_mpi(g));
                                packet.extend(bignum_to_mpi(y));
                            }
                            PublicParams::ECDSA { curve, p } => {
                                //a one-octet size of the following field
                                packet.push(curve.oid().len() as u8);
                                //octets representing a curve OID
                                packet.extend(curve.oid().iter().cloned());
                                //MPI of an EC point representing a public key
                                packet.extend(bignum_to_mpi(p));
                            }
                            PublicParams::ECDH {
                                curve,
                                p,
                                hash,
                                alg_sym,
                            } => {
                                //a one-octet size of the following field
                                packet.push(curve.oid().len() as u8);
                                //the octets representing a curve OID
                                packet.extend(curve.oid().iter().cloned());
                                //MPI of an EC point representing a public key
                                packet.extend(bignum_to_mpi(p));
                                //a one-octet size of the following fields
                                packet.push(3); // Always 3??
                                //a one-octet value 01
                                packet.push(1);
                                //a one-octet hash function ID used with a KDF
                                packet.push(*hash);
                                //a one-octet algorithm ID
                                packet.push(*alg_sym);
                            }
                            PublicParams::Elgamal { p, g, y } => {
                                packet.extend(bignum_to_mpi(p));
                                packet.extend(bignum_to_mpi(g));
                                packet.extend(bignum_to_mpi(y));
                            }
                        }

                        h.update(&packet).unwrap();

                        h.finish().unwrap().to_vec()
                    }
                }
            }

            pub fn key_id(&self) -> Option<Vec<u8>> {
                match self.version() {
                    KeyVersion::V4 => {
                        // Lower 64 bits
                        Some(self.fingerprint()[12..].to_vec())
                    }
                    KeyVersion::V2 | KeyVersion::V3 => match &self.public_params {
                        PublicParams::RSA { n, e: _ } => Some(n.to_vec()[12..].to_vec()),
                        _ => None,
                    },
                }
            }
        }
    };
}

key!(PublicKey);
key!(PrivateKey);
