use super::parse::read_packets;

use std::fs::File;

#[test]
fn read_demo_pub_ring() {
    let file = File::open("fixtures/demo/ring.pub").unwrap();
    let packets = read_packets(file).unwrap();

    println!("{:?}", packets);
}

#[test]
fn read_demo_ring() {
    let file = File::open("fixtures/demo/ring").unwrap();
    let packets = read_packets(file).unwrap();

    println!("{:?}", packets);
}
