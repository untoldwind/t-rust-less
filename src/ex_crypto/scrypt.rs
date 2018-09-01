use std;
use std::mem::size_of;

use openssl::pkcs5;
use openssl::error::ErrorStack;

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
    let n = 1 << params.log_n;
    pkcs5::scrypt(password, salt, n, params.r as u64, params.p as u64, 2 * 1024 * 1024 * 1024, output)?;
    Ok(())
}
