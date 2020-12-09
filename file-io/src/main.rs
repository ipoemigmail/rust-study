#[macro_use]
extern crate serde_json;

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::prelude::*;
use std::iter::FromIterator;
use std::path::Path;
use crc::{Crc, CRC_32_CKSUM};
use std::io::SeekFrom::Current;
use std::io::BufWriter;

pub static CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_CKSUM);

fn main() {
    let h: HashMap<_, _> = vec![
        ("key1", 1)
    ].into_iter().collect();
    let c = CRC32.checksum(&[1_u8]);
    let mut bw = BufWriter::new(vec!());
    let path = "./test";
    let maybe_f = File::open(path);
    let mut f = match maybe_f {
        Ok(f) => f,
        Err(_) => File::create(path).unwrap()
    };
    if let Err(_) = f.write_u32::<LittleEndian>(c) {
        panic!("error");
    }
    if let Err(_) = bw.write_u32::<LittleEndian>(c) {
        panic!("error");
    }
    println!("{:?}", bw.buffer());
    //let m: HashMap<_, _> = vec![("key1", 0), ("key2", 1)].into_iter().collect();
    //println!("{}", m["key1"]);
    //println!("{}", m["key2"]);
    println!("Hello, world!");
}
