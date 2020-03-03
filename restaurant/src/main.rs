use std::collections::*;
use rand::Rng;
use std::{io, cmp::Ordering};
use std::{self, write};

fn main() {
    let mut map = HashMap::new();
    map.insert(1, 2);

    let secret_number = rand::thread_rng().gen_range(1, 101);
}
