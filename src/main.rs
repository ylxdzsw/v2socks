#![allow(dead_code, unused_imports)]
#![deny(bare_trait_objects)]

use v2socks;
use oh_my_rust::*;
use std::net::TcpListener;
use std::io::{prelude::*, BufRead, BufReader};

fn main() {
    let socket = TcpListener::bind("0.0.0.0:1080").expect("Address already in use");
    info!("v2socks starts listening at 0.0.0.0:1080");

    for stream in socket.incoming() {
        let stream = stream.unwrap();
        std::thread::spawn(move || {
            if let Err(e) = initialize(&mut &stream) {
                error!("{}", e);
                return // close connection
            }

            std::io::copy(&mut &stream, &mut std::io::stdout()).unwrap();
        });
    }
}

fn initialize(stream: &mut (impl ReadExt + Write)) -> Result<(), String> {
    let mut header = [0, 0];
    stream.read_exact(&mut header).map_err(|_| "read initial bits failed")?;
    
    if header[0] != 5 {
        return Err(format!("unsupported socks version {}", header[0]))
    }

    let list: Vec<u8> = stream.read_exact_alloc(header[1] as usize).map_err(|_| "read methods failed")?;

    if !list.contains(&0) {
        stream.write(&[5, 0xff]).map_err(|_| "write response failed")?;
        return Err("client do not support NO AUTH method".to_owned())
    }

    stream.write(&[5, 0]).map_err(|_| "write response failed")?;
    Ok(())
}