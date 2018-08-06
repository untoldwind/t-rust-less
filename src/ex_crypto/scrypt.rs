// We use this implmentation as the OpenSSL implmentation has too strict memory limits.
// Might become obsolete once rust-openssl upgrades to OpenSSL 1.1
use std;
use std::{io, mem, ptr};
use std::mem::size_of;

use openssl::pkcs5::pbkdf2_hmac;
use openssl::hash::MessageDigest;
use openssl::error::ErrorStack;

/// Copy bytes from src to dest
#[inline]
pub fn copy_memory(src: &[u8], dst: &mut [u8]) {
    assert!(dst.len() >= src.len());
    unsafe {
        let srcp = src.as_ptr();
        let dstp = dst.as_mut_ptr();
        ptr::copy_nonoverlapping(srcp, dstp, src.len());
    }
}

/// Read the value of a vector of bytes as a u32 value in little-endian format.
pub fn read_u32_le(input: &[u8]) -> u32 {
    assert!(input.len() == 4);
    unsafe {
        let mut tmp: u32 = mem::uninitialized();
        ptr::copy_nonoverlapping(input.get_unchecked(0), &mut tmp as *mut _ as *mut u8, 4);
        u32::from_le(tmp)
    }
}

/// Write a u32 into a vector, which must be 4 bytes long. The value is written in little-endian
/// format.
pub fn write_u32_le(dst: &mut [u8], mut input: u32) {
    assert!(dst.len() == 4);
    input = input.to_le();
    unsafe {
        let tmp = &input as *const _ as *const u8;
        ptr::copy_nonoverlapping(tmp, dst.get_unchecked_mut(0), 4);
    }
}

/// Read a vector of bytes into a vector of u32s. The values are read in little-endian format.
pub fn read_u32v_le(dst: &mut [u32], input: &[u8]) {
    assert!(dst.len() * 4 == input.len());
    unsafe {
        let mut x: *mut u32 = dst.get_unchecked_mut(0);
        let mut y: *const u8 = input.get_unchecked(0);
        for _ in 0..dst.len() {
            let mut tmp: u32 = mem::uninitialized();
            ptr::copy_nonoverlapping(y, &mut tmp as *mut _ as *mut u8, 4);
            *x = u32::from_le(tmp);
            x = x.offset(1);
            y = y.offset(4);
        }
    }
}

// The salsa20/8 core function.
fn salsa20_8(input: &[u8], output: &mut [u8]) {
    let mut x = [0u32; 16];
    read_u32v_le(&mut x, input);

    let rounds = 8;

    macro_rules! run_round (
        ($($set_idx:expr, $idx_a:expr, $idx_b:expr, $rot:expr);*) => { {
            $( x[$set_idx] ^= x[$idx_a].wrapping_add(x[$idx_b]).rotate_left($rot); )*
        } }
    );

    for _ in 0..rounds / 2 {
        run_round!(
            0x4, 0x0, 0xc, 7;
            0x8, 0x4, 0x0, 9;
            0xc, 0x8, 0x4, 13;
            0x0, 0xc, 0x8, 18;
            0x9, 0x5, 0x1, 7;
            0xd, 0x9, 0x5, 9;
            0x1, 0xd, 0x9, 13;
            0x5, 0x1, 0xd, 18;
            0xe, 0xa, 0x6, 7;
            0x2, 0xe, 0xa, 9;
            0x6, 0x2, 0xe, 13;
            0xa, 0x6, 0x2, 18;
            0x3, 0xf, 0xb, 7;
            0x7, 0x3, 0xf, 9;
            0xb, 0x7, 0x3, 13;
            0xf, 0xb, 0x7, 18;
            0x1, 0x0, 0x3, 7;
            0x2, 0x1, 0x0, 9;
            0x3, 0x2, 0x1, 13;
            0x0, 0x3, 0x2, 18;
            0x6, 0x5, 0x4, 7;
            0x7, 0x6, 0x5, 9;
            0x4, 0x7, 0x6, 13;
            0x5, 0x4, 0x7, 18;
            0xb, 0xa, 0x9, 7;
            0x8, 0xb, 0xa, 9;
            0x9, 0x8, 0xb, 13;
            0xa, 0x9, 0x8, 18;
            0xc, 0xf, 0xe, 7;
            0xd, 0xc, 0xf, 9;
            0xe, 0xd, 0xc, 13;
            0xf, 0xe, 0xd, 18
        )
    }

    for i in 0..16 {
        write_u32_le(
            &mut output[i * 4..(i + 1) * 4],
            x[i].wrapping_add(read_u32_le(&input[i * 4..(i + 1) * 4])),
        );
    }
}

fn xor(x: &[u8], y: &[u8], output: &mut [u8]) {
    for ((out, &x_i), &y_i) in output.iter_mut().zip(x.iter()).zip(y.iter()) {
        *out = x_i ^ y_i;
    }
}

// Execute the BlockMix operation
// input - the input vector. The length must be a multiple of 128.
// output - the output vector. Must be the same length as input.
fn scrypt_block_mix(input: &[u8], output: &mut [u8]) {
    let mut x = [0u8; 64];
    copy_memory(&input[input.len() - 64..], &mut x);

    let mut t = [0u8; 64];

    for (i, chunk) in input.chunks(64).enumerate() {
        xor(&x, chunk, &mut t);
        salsa20_8(&t, &mut x);
        let pos = if i % 2 == 0 {
            (i / 2) * 64
        } else {
            (i / 2) * 64 + input.len() / 2
        };
        copy_memory(&x, &mut output[pos..pos + 64]);
    }
}

// Execute the ROMix operation in-place.
// b - the data to operate on
// v - a temporary variable to store the vector V
// t - a temporary variable to store the result of the xor
// n - the scrypt parameter N
fn scrypt_ro_mix(b: &mut [u8], v: &mut [u8], t: &mut [u8], n: usize) {
    fn integerify(x: &[u8], n: usize) -> usize {
        // n is a power of 2, so n - 1 gives us a bitmask that we can use to perform a calculation
        // mod n using a simple bitwise and.
        let mask = n - 1;
        // This cast is safe since we're going to get the value mod n (which is a power of 2), so we
        // don't have to care about truncating any of the high bits off
        let result = (read_u32_le(&x[x.len() - 64..x.len() - 60]) as usize) & mask;
        result
    }

    let len = b.len();

    for chunk in v.chunks_mut(len) {
        copy_memory(b, chunk);
        scrypt_block_mix(chunk, b);
    }

    for _ in 0..n {
        let j = integerify(b, n);
        xor(b, &v[j * len..(j + 1) * len], t);
        scrypt_block_mix(t, b);
    }
}

/**
 * The Scrypt parameter values.
 */
#[derive(Clone, Copy)]
pub struct ScryptParams {
    log_n: u8,
    r: u32,
    p: u32,
}

impl ScryptParams {
    /**
     * Create a new instance of ScryptParams.
     *
     * # Arguments
     *
     * * log_n - The log2 of the Scrypt parameter N
     * * r - The Scrypt parameter r
     * * p - The Scrypt parameter p
     *
     */
    pub fn new(log_n: u8, r: u32, p: u32) -> ScryptParams {
        assert!(r > 0);
        assert!(p > 0);
        assert!(log_n > 0);
        assert!((log_n as usize) < size_of::<usize>() * 8);
        assert!(size_of::<usize>() >= size_of::<u32>() || (r <= std::usize::MAX as u32 && p < std::usize::MAX as u32));

        let r = r as usize;
        let p = p as usize;

        let n: usize = 1 << log_n;

        // check that r * 128 doesn't overflow
        let r128 = match r.checked_mul(128) {
            Some(x) => x,
            None => panic!("Invalid Scrypt parameters."),
        };

        // check that n * r * 128 doesn't overflow
        match r128.checked_mul(n) {
            Some(_) => {}
            None => panic!("Invalid Scrypt parameters."),
        };

        // check that p * r * 128 doesn't overflow
        match r128.checked_mul(p) {
            Some(_) => {}
            None => panic!("Invalid Scrypt parameters."),
        };

        // This check required by Scrypt:
        // check: n < 2^(128 * r / 8)
        // r * 16 won't overflow since r128 didn't
        assert!((log_n as usize) < r * 16);

        // This check required by Scrypt:
        // check: p <= ((2^32-1) * 32) / (128 * r)
        // It takes a bit of re-arranging to get the check above into this form, but, it is indeed
        // the same.
        assert!(r * p < 0x40000000);

        ScryptParams {
            log_n: log_n,
            r: r as u32,
            p: p as u32,
        }
    }
}

/**
 * The scrypt key derivation function.
 *
 * # Arguments
 *
 * * `password` - The password to process as a byte vector
 * * `salt` - The salt value to use as a byte vector
 * * `params` - The `ScryptParams` to use
 * * `output` - The resulting derived key is returned in this byte vector.
 *
 */
pub fn scrypt(password: &[u8], salt: &[u8], params: &ScryptParams, output: &mut [u8]) -> Result<(), ErrorStack> {
    // This check required by Scrypt:
    // check output.len() > 0 && output.len() <= (2^32 - 1) * 32
    assert!(!output.is_empty());
    assert!(output.len() / 32 <= 0xffffffff);

    // The checks in the ScryptParams constructor guarantee that the following is safe:
    let n = 1 << params.log_n;
    let r128 = (params.r as usize) * 128;
    let pr128 = (params.p as usize) * r128;
    let nr128 = n * r128;

    let mut b: Vec<u8> = vec![0; pr128];
    pbkdf2_hmac(password, salt, 1, MessageDigest::sha256(), &mut b)?;

    let mut v: Vec<u8> = vec![0; nr128];
    let mut t: Vec<u8> = vec![0; r128];

    for chunk in &mut b.chunks_mut(r128) {
        scrypt_ro_mix(chunk, &mut v, &mut t, n);
    }

    pbkdf2_hmac(password, &b, 1, MessageDigest::sha256(), output)?;

    Ok(())
}
