use super::*;
use oh_my_rust::*;
use std::net::{TcpListener, TcpStream};
use std::io::{prelude::*, BufRead, BufReader};

macro_rules! read_exact {
    ($stream: expr, $array: expr) => {{
        let mut x = $array;
        $stream.read_exact(&mut x).map(|_| x)
    }}
}

macro_rules! close_on_error {
    ($ex: expr) => {{
        match $ex {
            Ok(x) => x,
            Err(e) => {
                return error!("{}", e)
            }
        }
    }}
}

pub struct Socks5Server {
    port: u16
}

impl Socks5Server {
    pub fn new(port: u16) -> Socks5Server {
        Socks5Server { port }
    }

    pub fn listen<T>(&self, connect: &'static (impl Fn(Addr, u16) -> std::io::Result<(Addr, u16, T)> + Sync), pass: &'static (impl Fn(T, TcpStream) + Sync)) {
        let socket = TcpListener::bind(format!("0.0.0.0:{}", self.port)).expect("Address already in use");
        info!("v2socks starts listening at 0.0.0.0:{}", self.port);

        for stream in socket.incoming() {
            let stream = stream.unwrap();
            std::thread::spawn(move || {
                close_on_error!(initialize(&mut &stream));
                let (addr, port) = close_on_error!(read_request(&mut &stream)); // TODO: properly respond with correct error number
                let (local_addr, local_port, proxy) = close_on_error!(connect(addr, port));
                close_on_error!(reply_request(&mut &stream, local_addr, local_port));
                pass(proxy, stream);
            });
        }
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

fn read_request(stream: &mut (impl ReadExt + Write)) -> Result<(Addr, u16), String> {
    let mut header = [0; 4];
    stream.read_exact(&mut header).map_err(|_| "read request header failed")?;
    let [ver, cmd, _rev, atyp] = header;

    if ver != 5 {
        return Err(format!("unsupported socks version {}", ver))
    }

    if cmd != 1 {
        return Err(format!("unsupported command type {}", cmd))
    }

    let addr = match atyp {
        0x01 => Addr::V4(read_exact!(stream, [0; 4]).map_err(|_| "read v4 address failed")?),
        0x04 => Addr::V6(read_exact!(stream, [0; 16]).map_err(|_| "read v6 address failed")?),
        0x03 => {
            let mut len = [0];
            stream.read_exact(&mut len).map_err(|_| "read domain length failed")?;
            let len = len[0];

            Addr::Domain(stream.read_exact_alloc(len as usize).map_err(|_| "read domain failed")?.into_boxed_slice())
        },
        _ => return Err("unknown ATYP".to_owned())
    };

    let mut port: [u8; 2] = [0; 2];
    stream.read_exact(&mut port).map_err(|_| "read port failed")?;
    let port: u16 = (port[0] as u16) << 8 | port[1] as u16;

    Ok((addr, port))
}

fn reply_request(stream: &mut (impl ReadExt + Write), addr: Addr, port: u16) -> Result<(), String> {
    let mut reply = vec![5, 0, 0];

    match addr {
        Addr::V4(x) => {
            reply.push(1);
            reply.extend_from_slice(&x);
        },
        Addr::V6(x) => {
            reply.push(4);
            reply.extend_from_slice(&x);
        },
        Addr::Domain(x) => {
            reply.push(3);
            reply.push(x.len() as u8);
            reply.extend_from_slice(&x);
        }
    }

    reply.push((port >> 8) as u8);
    reply.push(port as u8);

    stream.write(&reply).map_err(|_| "write reply failed")?;

    Ok(())
}

