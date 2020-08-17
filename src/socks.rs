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
                return warn!("{}", e)
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

    pub fn listen<T>(&self, connect: &'static (impl Fn(Addr, u16) -> std::io::Result<(Addr, u16, T)> + Sync), pass: &'static (impl Fn(T, TcpStream) + Sync)) -> ! {
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

        unreachable!()
    }
}

fn initialize(stream: &mut (impl ReadExt + Write)) -> Result<(), String> {
    let header = read_exact!(stream, [0, 0]).msg("read initial bits failed")?;

    if header[0] != 5 {
        let hint = "if the version is 71, the software probabily used it as an HTTP proxy";
        return Err(format!("unsupported socks version {}. Hint: {}", header[0], hint))
    }

    let list: Vec<u8> = stream.read_exact_alloc(header[1] as usize).msg("read methods failed")?;

    if !list.contains(&0) {
        stream.write(&[5, 0xff]).msg("write response failed")?;
        return Err("client do not support NO AUTH method".to_string())
    }

    stream.write(&[5, 0]).msg("write response failed")?;
    Ok(())
}

fn read_request(stream: &mut (impl ReadExt + Write)) -> Result<(Addr, u16), String> {
    let [ver, cmd, _rev, atyp] = read_exact!(stream, [0; 4]).msg("read request header failed")?;

    if ver != 5 {
        return Err(format!("unsupported socks version {}", ver))
    }

    if cmd != 1 {
        return Err(format!("unsupported command type {}", cmd))
    }

    let addr = match atyp {
        0x01 => Addr::V4(read_exact!(stream, [0; 4]).msg("read v4 address failed")?),
        0x04 => Addr::V6(read_exact!(stream, [0; 16]).msg("read v6 address failed")?),
        0x03 => {
            let len = read_exact!(stream, [0]).msg("read domain length failed")?[0];
            Addr::Domain(stream.read_exact_alloc(len as usize).msg("read domain failed")?.into_boxed_slice())
        },
        _ => return Err("unknown ATYP".to_string())
    };

    let port = read_exact!(stream, [0; 2]).msg("read port failed")?;
    let port = (port[0] as u16) << 8 | port[1] as u16;

    Ok((addr, port))
}

fn reply_request(stream: &mut (impl ReadExt + Write), addr: Addr, port: u16) -> Result<(), String> {
    let mut reply = Vec::with_capacity(22); // cover V4 and V6
    reply.extend_from_slice(&[5, 0, 0]);

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

    stream.write(&reply).msg("write reply failed")?;

    Ok(())
}

