use num_traits::FromPrimitive;

use super::{Packet, Tag, Version};
use super::util::{u16_as_usize, u32_as_usize, u8_as_usize};
use nom::{self, Needed, Offset};
use circular::Buffer;
use std::io::Read;
use ex_crypto::error::Result;

/// Parses an old format packet header
/// Ref: https://tools.ietf.org/html/rfc4880.html#section-4.2.1
named!(old_packet_header(&[u8]) -> (Version, Tag, usize), bits!(do_parse!(
    // First bit is always 1
            tag_bits!(u8, 1, 1)
    // Version: 0
    >> ver: map_opt!(tag_bits!(u8, 1, 0), Version::from_u8)
    // Packet Tag
    >> tag: map_opt!(take_bits!(u8, 4), Tag::from_u8)
    // Packet Length Type
    >> len_type: take_bits!(u8, 2)
    >> len: switch!(value!(len_type),
        // One-Octet Lengths
        0 => map!(take_bits!(u8, 8), u8_as_usize)    |
        // Two-Octet Lengths
        1 => map!(take_bits!(u16, 16), u16_as_usize) |
        // Four-Octet Lengths
        2 => map!(take_bits!(u32, 32), u32_as_usize)
    )
        >> (ver, tag, len)
)));

/// Parses a new format packet header
/// Ref: https://tools.ietf.org/html/rfc4880.html#section-4.2.2
named!(new_packet_header(&[u8]) -> (Version, Tag, usize), bits!(do_parse!(
    // First bit is always 1
             tag_bits!(u8, 1, 1)
    // Version: 1
    >>  ver: map_opt!(tag_bits!(u8, 1, 1), Version::from_u8)
    // Packet Tag
    >>  tag: map_opt!(take_bits!(u8, 6), Tag::from_u8)
    >> olen: take_bits!(u8, 8)
    >>  len: switch!(value!(olen),
        // One-Octet Lengths
        0...191   => value!(olen as usize) |
        // Two-Octet Lengths
        192...254 => map!(take_bits!(u8, 8), |a| {
            ((olen as usize - 192) << 8) + 192 + a as usize
        }) |
        // Five-Octet Lengths
        255       => map!(take_bits!(u32, 32), u32_as_usize)
        // Partial Body Lengths
        // TODO: 224...254 => value!(1)
    )
    >> (ver, tag, len)
)));

/// Parse Packet Headers
/// ref: https://tools.ietf.org/html/rfc4880.html#section-4.2
named!(pub parser<Packet>, do_parse!(
       head: alt!(new_packet_header | old_packet_header)
    >> body: take!(head.2)
    >> packet: expr_res!(Packet::new(head.0, head.1, body.to_vec()))
    >> (packet)
));

/// Parse packets, in a streaming fashion from the given reader.
pub fn read_packets(mut input: impl Read) -> Result<Vec<Packet>> {
    // maximum size of our buffer
    let max_capacity = 1024 * 1024 * 1024;
    // the inital capacity of our buffer
    // TODO: use a better value than a random guess
    let mut capacity = 1024;
    let mut b = Buffer::with_capacity(capacity);

    let mut packets = Vec::new();

    loop {
        // read some data
        let sz = input.read(b.space()).unwrap();
        b.fill(sz);

        // if there's no more available data in the buffer after a write, that means we reached
        // the end of the input
        if b.available_data() == 0 {
            break;
        }

        let needed: Option<Needed>;

        loop {
            let length = {
                match parser(b.data()) {
                    Ok((remaining, p)) => {
                        packets.push(p);
                        b.data().offset(remaining)
                    }
                    Err(err) => match err {
                        nom::Err::Incomplete(n) => {
                            needed = Some(n);
                            break;
                        }
                        _ => return Err(err.into()),
                    },
                }
            };

            b.consume(length);
        }

        // if the parser returned `Incomplete`, and it needs more data than the buffer can hold, we grow the buffer.
        if let Some(Needed::Size(sz)) = needed {
            if sz > b.capacity() && capacity * 2 < max_capacity {
                capacity *= 2;
                b.grow(capacity);
            }
        }
    }

    Ok(packets)
}
