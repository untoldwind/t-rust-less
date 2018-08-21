use super::parse::read_packets;
use super::Packet;
use data_encoding::HEXUPPER;

use std::fs::File;

#[test]
fn read_demo_pub_ring() {
    let file = File::open("fixtures/demo/ring.pub").unwrap();
    let packets = read_packets(file).unwrap();

    for packet in packets {
        println!("{:?}", packet);
        match packet {
            Packet::PublicKey(key) => println!("{}", HEXUPPER.encode(&key.key_id().unwrap())),
            _ => (),
        }
    }
}

#[test]
fn read_demo_ring() {
    let file = File::open("fixtures/demo/ring").unwrap();
    let packets = read_packets(file).unwrap();

    for packet in packets {
        println!("{:?}", packet);
        match packet {
            Packet::PublicKey(key) => println!("{}", HEXUPPER.encode(&key.key_id().unwrap())),
            _ => (),
        }
    }
}
