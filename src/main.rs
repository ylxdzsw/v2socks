#![allow(dead_code, unused_imports)]
#![deny(bare_trait_objects)]
#![allow(non_camel_case_types)]

use v2socks;
use oh_my_rust::*;
use std::net::TcpListener;
use std::io::{prelude::*, BufRead, BufReader};

#[derive(Debug)]
enum ADDR {
    V4(Box<[u8]>),
    V6(Box<[u8]>),
    Domain(Box<[u8]>)
}

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

            let req = match read_request(&mut &stream) {
                Ok(req) => req,
                Err(e) => {
                    error!("{}", e);
                    // TODO: properly respond with correct error number
                    return // close connection
                }
            };

            debug!("{:?}", req);

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

fn read_request(stream: &mut (impl ReadExt + Write)) -> Result<(u8, ADDR, u16), String> {
    let mut header = [0; 4];
    stream.read_exact(&mut header).map_err(|_| "read request header failed")?;
    let [ver, cmd, _rev, atyp] = header;

    if ver != 5 {
        return Err(format!("unsupported socks version {}", header[0]))
    }

    let addr = match atyp {
        0x01 => ADDR::V4(stream.read_exact_alloc(4).map_err(|_| "read v4 address failed")?.into_boxed_slice()),
        0x04 => ADDR::V6(stream.read_exact_alloc(16).map_err(|_| "read v6 address failed")?.into_boxed_slice()),
        0x03 => {
            let mut len = [0];
            stream.read_exact(&mut len).map_err(|_| "read domain length failed")?;
            let len = len[0];

            ADDR::Domain(stream.read_exact_alloc(len as usize).map_err(|_| "read domain failed")?.into_boxed_slice())
        },
        _ => return Err("unknown ATYP".to_owned())
    };

    let mut port: [u8; 2] = [0; 2];
    stream.read_exact(&mut port).map_err(|_| "read port failed")?;
    let port: u16 = (port[0] as u16) << 8 | port[1] as u16;

    Ok((cmd, addr, port))
}