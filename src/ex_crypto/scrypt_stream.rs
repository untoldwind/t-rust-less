use std::io::{Read, Write, Result};
use std::mem;
use std::slice;

#[repr(C, packed)]
#[derive(Debug)]
struct Params {
    log_n: u8,
    r: u32,
    p: u32,
}

#[repr(C, packed)]
#[derive(Debug)]
struct Header {
    magic: [u8; 6],
    version: u8,
    params: Params,
    salt: [u8; 32],
    header_hash: [u8; 32],
    header_hmac: [u8; 32],
}

pub fn decrypt(password: &[u8], from: &mut Read, to: &Write) -> Result<()> {
    let mut header: Header = unsafe { mem::zeroed() };
    let header_size = mem::size_of::<Header>();

    unsafe {
        let header_slice = slice::from_raw_parts_mut(
            &mut header as *mut _ as *mut u8,
            header_size
        );
        // `read_exact()` comes from `Read` impl for `&[u8]`
        from.read_exact(header_slice)?;
    }
    println!("{:?}", header);

    Ok(())
}
