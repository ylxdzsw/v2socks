#![allow(dead_code, unused_imports)]
#![deny(bare_trait_objects)]

use v2socks;
use crypto::digest::Digest;
use crypto::sha3::Sha3;

fn main() {
    let mut hasher = Sha3::shake_128();
    hasher.input(b" fuck them");
    let hex = hasher.result_str();
    println!("Hello, world! and {}", hex);
}
