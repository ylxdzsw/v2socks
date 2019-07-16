use v2socks::*;
use oh_my_rust::*;
use rand::prelude::*;
use std::io::prelude::*;

// basic logic:
// 1. main thread listen for socks5 connections
// 2. spawn a child thread for each connection, perform the sock5 and vmess handshake respectively
// 3. after the handshake succeed, spawn a pair of threads to pipe the two connections forward and backward

// todo: use thread pool or async io

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let args: Vec<_> = args.iter().map(|x| &x[..]).collect();
    match args[..] {
        ["plain"] | ["plain", _] => {
            let port: u16 = args.get(1).map(|x| x.parse().unwrap()).unwrap_or(1080);
            let server = Socks5Server::new(port);
            plain(&server)
        },
        ["vmess", proxy, user_id] | ["vmess", proxy, user_id, _] => {
            let port: u16 = args.get(4).map(|x| x.parse().unwrap()).unwrap_or(1080);
            let server = Socks5Server::new(port);
            vmess(&server, proxy.into(), parse_uid(user_id).unwrap())
        },
        _ => {
            eprintln!("
                Usage: v2socks plain [local_port=1080]
                       v2socks vmess <server_addr>:<server_port> <userid> [local_port=1080]
            ")
        },
    }
}

fn parse_uid(x: &str) -> Option<[u8; 16]> {
    let mut r = [0; 16];
    let x = x.replace('-', "");
    let list: Vec<_> = (0..32).step_by(2).map(|i| u8::from_str_radix(&x[i..i+2], 16).unwrap()).collect();
    r.clone_from_slice(list.get(0..16)?);
    Some(r)
}

fn is_normal_close(e: &std::io::Error) -> bool {
    match e.kind() {
        std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset => true,
        _ => false
    }
}

fn vmess(server: &Socks5Server, proxy: String, user_id: [u8; 16]) {
    let connect = Box::leak(Box::new(move |dest, port| {
        let client = std::net::TcpStream::connect(&proxy)?;
        debug!("connect {}:{} through proxy", &dest, port);

        let local = client.local_addr()?;
        let local_addr = match local.ip() {
            std::net::IpAddr::V4(x) => Addr::V4(x.octets()),
            std::net::IpAddr::V6(x) => Addr::V6(x.octets()),
        };
        let local_port = local.port();

        Ok((local_addr, local_port, (dest, port, client)))
    }));

    #[allow(non_snake_case)]
    let pass = Box::leak(Box::new(move |(dest, port, conn): (Addr, u16, std::net::TcpStream), mut stream: std::net::TcpStream| {
        let mut key = [0; 16];
        thread_rng().fill_bytes(&mut key);

        let mut IV = [0; 16];
        thread_rng().fill_bytes(&mut IV);

        {
            let conn = conn.try_clone().expect("failed to clone TCP handle");
            let mut stream = stream.try_clone().expect("failed to clone TCP handle");

            std::thread::spawn(move || {
                let mut buffer = Box::new( unsafe { std::mem::uninitialized::<[u8; 16384]>() } );
                let mut reader = match VmessReader::new(conn, key, IV) {
                    Some(x) => x,
                    None => return warn!("reader handshake failed")
                };
                loop {
                    let len = match reader.read(&mut *buffer) {
                        Ok(0) => break,
                        Ok(x) => x,
                        Err(ref e) if is_normal_close(e) => break,
                        Err(e) => { error!("{}", e); break }
                    };

                    match stream.write_all(&buffer[..len]) {
                        Ok(_) => debug!("read {} bytes", len),
                        Err(ref e) if is_normal_close(e) => break,
                        Err(e) => { error!("{}", e); break }
                    }
                }
                reader.close();
                debug!("closed reading")
            });
        }

        let mut buffer = Box::new( unsafe { std::mem::uninitialized::<[u8; 16384]>() } );
        let mut writer = match VmessWriter::new(conn, user_id, dest, port, key, IV) {
            Some(x) => x,
            None => return warn!("writer handshake failed")
        };
        loop {
            let len = match stream.read(&mut *buffer) {
                Ok(0) => break,
                Ok(x) => x,
                Err(ref e) if is_normal_close(e) => break,
                Err(e) => { error!("{}", e); break }
            };

            match writer.write_all(&buffer[..len]) {
                Ok(_) => debug!("sent {} bytes", len),
                Err(ref e) if is_normal_close(e) => break,
                Err(e) => { error!("{}", e); break }
            }
        }

        writer.close();
        debug!("closed writing");
    }));

    server.listen(connect, pass)
}

fn plain(server: &Socks5Server) {
    server.listen(&|dest, port| {
        let client = std::net::TcpStream::connect(format!("{}:{}", dest, port))?;
        debug!("connect {}:{}", dest, port);

        let local = client.local_addr()?;
        let local_addr = match local.ip() {
            std::net::IpAddr::V4(x) => Addr::V4(x.octets()),
            std::net::IpAddr::V6(x) => Addr::V6(x.octets()),
        };
        let local_port = local.port();

        Ok((local_addr, local_port, client))
    }, &|mut proxy, mut stream| {
        {
            let mut proxy = proxy.try_clone().expect("failed to clone TCP handle");
            let mut stream = stream.try_clone().expect("failed to clone TCP handle");
            std::thread::spawn(move || {
                std::io::copy(&mut proxy, &mut stream).ignore()
            });
        }
        std::io::copy(&mut stream, &mut proxy).ignore();
    })
}
