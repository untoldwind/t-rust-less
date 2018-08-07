use std::io::{Read, Write};
use std::time::Instant;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use openssl::hash::{DigestBytes, Hasher, MessageDigest};
use openssl::sign::Signer;
use openssl::symm::{Cipher, Crypter, Mode};
use openssl::pkey::{PKey, Private};
use openssl::memcmp::eq;
use openssl::rand::rand_bytes;
use super::error::{ErrorKind, Result};
use super::scrypt::{scrypt, ScryptParams};

const MAGIC: [u8; 6] = *b"scrypt";

#[derive(Debug)]
pub struct Params {
    log_n: u8,
    r: u32,
    p: u32,
}

impl Params {
    fn read(from: &mut Read) -> Result<Params> {
        let log_n = from.read_u8()?;
        let r = from.read_u32::<BigEndian>()?;
        let p = from.read_u32::<BigEndian>()?;
        Ok(Params { log_n, r, p })
    }

    fn write(&self, to: &mut Write) -> Result<()> {
        to.write_u8(self.log_n)?;
        to.write_u32::<BigEndian>(self.r)?;
        to.write_u32::<BigEndian>(self.p)?;
        Ok(())
    }
}

#[derive(Debug)]
struct Header {
    magic: [u8; 6],
    version: u8,
    params: Params,
    salt: [u8; 32],
    header_hash: [u8; 16],
    header_hmac: [u8; 32],
}

impl Header {
    fn new(params: Params) -> Result<Header> {
        let mut salt: [u8; 32] = [0; 32];
        rand_bytes(&mut salt)?;
        let mut header = Header {
            magic: MAGIC,
            version: 0,
            params,
            salt,
            header_hash: [0; 16],
            header_hmac: [0; 32],
        };
        let header_hash = header.calc_header_hash()?;
        header.header_hash.copy_from_slice(&header_hash[..16]);

        Ok(header)
    }

    fn read(from: &mut Read) -> Result<Header> {
        let mut magic: [u8; 6] = [0; 6];
        from.read_exact(&mut magic)?;
        let version = from.read_u8()?;
        let params = Params::read(from)?;
        let mut salt: [u8; 32] = [0; 32];
        from.read_exact(&mut salt)?;
        let mut header_hash: [u8; 16] = [0; 16];
        from.read_exact(&mut header_hash)?;
        let mut header_hmac: [u8; 32] = [0; 32];
        from.read_exact(&mut header_hmac)?;
        Ok(Header {
            magic,
            version,
            params,
            salt,
            header_hash,
            header_hmac,
        })
    }

    fn write_head(&self, to: &mut Write) -> Result<()> {
        to.write(&self.magic[..])?;
        to.write_u8(self.version)?;
        self.params.write(to)?;
        to.write(&self.salt[..])?;
        Ok(())
    }

    fn write_with_hash(&self, to: &mut Write) -> Result<()> {
        self.write_head(to)?;
        to.write(&self.header_hash[..])?;
        Ok(())
    }

    fn write(&self, to: &mut Write) -> Result<()> {
        self.write_with_hash(to)?;
        to.write(&self.header_hmac[..])?;
        Ok(())
    }

    fn calc_header_hash(&self) -> Result<DigestBytes> {
        let mut sha256 = Hasher::new(MessageDigest::sha256())?;

        self.write_head(&mut sha256)?;
        Ok(sha256.finish()?)
    }

    fn calc_header_hmac(&self, hmac_key: &PKey<Private>) -> Result<Vec<u8>> {
        let mut signer = Signer::new(MessageDigest::sha256(), &hmac_key)?;

        self.write_with_hash(&mut signer)?;
        Ok(signer.sign_to_vec()?)
    }
}

fn cpuperf() -> Result<u64> {
    let params = ScryptParams::new(7, 1, 1);
    let mut target: [u8; 1] = [0; 1];
    let start = Instant::now();
    let mut iterations: u64 = 0;
    let mut diff: u64;

    loop {
        scrypt(b"", b"", &params, &mut target)?;
        iterations += 512;
        let elapsed = start.elapsed();
        diff = elapsed.as_secs() * 1000 + elapsed.subsec_nanos() as u64 / 1_000_000;
        if diff > 10 {
            break;
        }
    }
    Ok(iterations * 1000 / diff)
}

fn pickparams(memlimit: u64, maxtime: f64) -> Result<Params> {
    let ops_per_second = cpuperf()?;
    let mut ops_limit = ops_per_second as f64 * maxtime;
    if ops_limit < 32768f64 {
        ops_limit = 32768f64
    }
    let r = 8u32;
    if ops_limit < (memlimit / 32) as f64 {
        let max_n = ops_limit / (r * 4) as f64;
        let mut log_n = 1u8;
        loop {
            if log_n >= 63 || (2 << log_n) as f64 > max_n {
                break;
            }
            log_n += 1;
        }
        Ok(Params { log_n, r, p: 1 })
    } else {
        let max_n = memlimit / (r * 120) as u64;
        let mut log_n = 1u8;
        loop {
            if log_n >= 63 || (2u64 << log_n) > max_n {
                break;
            }
            log_n += 1;
        }
        let mut maxrp = (ops_limit / 4f64) as u64 / (1u64 << log_n);
        if maxrp > 0x3fffffffu64 {
            maxrp = 0x3fffffffu64
        }
        Ok(Params {
            log_n,
            r,
            p: maxrp as u32,
        })
    }
}

pub fn encrypt(password: &[u8], from: &mut Read, to: &mut Write, maybe_params: Option<Params>) -> Result<()> {
    let params = maybe_params
        .ok_or("Missing params")
        .or_else(|_| pickparams(1024 * 1024 * 1024, 0.5))?;
    let scrypt_params = ScryptParams::new(params.log_n, params.r, params.p);
    let mut header = Header::new(params)?;
    let mut dk: Vec<u8> = vec![0; 64];

    scrypt(password, &header.salt, &scrypt_params, &mut dk)?;

    let hmac_key = PKey::hmac(&dk[32..64])?;
    let mut signer = Signer::new(MessageDigest::sha256(), &hmac_key)?;
    let header_hmac = header.calc_header_hmac(&hmac_key)?;
    header.header_hmac.copy_from_slice(&header_hmac);
    header.write(to)?;
    header.write(&mut signer)?;

    let mut in_buf: [u8; 8160] = [0; 8160];
    let mut out_buf: [u8; 8192] = [0; 8192];
    let mut crypter = Crypter::new(
        Cipher::aes_256_ctr(),
        Mode::Encrypt,
        &dk[0..32],
        Some(&[0; 16]),
    )?;

    loop {
        let len = match from.read(&mut in_buf) {
            Ok(0) => break,
            Ok(len) => len,
            Err(ref e) if e.kind() == ::std::io::ErrorKind::Interrupted => continue,
            Err(e) => bail!(e),
        };
        let n = crypter.update(&in_buf[0..len], &mut out_buf)?;
        to.write(&out_buf[0..n])?;
        signer.write(&out_buf[0..n])?;
    }
    let content_hmac = signer.sign_to_vec()?;
    to.write(&content_hmac)?;

    Ok(())
}

pub fn decrypt(password: &[u8], from: &mut Read, to: &mut Write) -> Result<()> {
    let header = Header::read(from)?;

    if header.magic != MAGIC {
        bail!(ErrorKind::InvalidHeader(format!(
            "Invalid magic: {:?}",
            header.magic
        )))
    }
    if header.version != 0 {
        bail!(ErrorKind::InvalidHeader(format!(
            "Invalid version: {}",
            header.version
        )))
    }

    let header_hash = &header.calc_header_hash()?[..16];
    if !eq(&header.header_hash, header_hash) {
        bail!(ErrorKind::InvalidHeader(format!(
            "Header checksum does not match: {:?} {:?}",
            header.header_hash, &header_hash
        )))
    }

    let params = ScryptParams::new(header.params.log_n, header.params.r, header.params.p);
    let mut dk: Vec<u8> = vec![0; 64];

    scrypt(password, &header.salt, &params, &mut dk)?;

    let hmac_key = PKey::hmac(&dk[32..64])?;
    let header_hmac = header.calc_header_hmac(&hmac_key)?;

    if !eq(&header.header_hmac, &header_hmac) {
        bail!(ErrorKind::InvalidHeader(
            "Header HMAC does not match".to_string()
        ))
    }

    let mut signer = Signer::new(MessageDigest::sha256(), &hmac_key)?;
    header.write(&mut signer)?;
    let mut crypter = Crypter::new(
        Cipher::aes_256_ctr(),
        Mode::Decrypt,
        &dk[0..32],
        Some(&[0; 16]),
    )?;
    let mut in_buf: [u8; 8192] = [0; 8192];
    let mut out_buf: [u8; 8192] = [0; 8192];
    let mut buf_length: usize = 0;

    loop {
        let len = match from.read(&mut in_buf[buf_length..]) {
            Ok(0) => break,
            Ok(len) => len,
            Err(ref e) if e.kind() == ::std::io::ErrorKind::Interrupted => continue,
            Err(e) => bail!(e),
        };
        buf_length += len;
        if buf_length > 32 {
            signer.write(&in_buf[0..buf_length - 32])?;
            let n = crypter.update(&in_buf[0..buf_length - 32], &mut out_buf)?;
            to.write(&out_buf[0..n])?;
            for i in 0..32 {
                in_buf[i] = in_buf[buf_length - 32 + i]
            }
            buf_length = 32
        }
    }
    if buf_length < 32 {
        bail!(ErrorKind::InvalidContent(
            "Content to short, missing HMAC".to_string()
        ))
    }
    let content_hmac = signer.sign_to_vec()?;
    if !eq(&in_buf[0..32], &content_hmac) {
        bail!(ErrorKind::InvalidContent(
            "Content HMAC does not match".to_string()
        ))
    }

    Ok(())
}
