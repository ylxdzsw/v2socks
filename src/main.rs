use v2socks::*;
use oh_my_rust::*;
use rand::prelude::*;
use std::io::prelude::*;

// basic logic:
// 1. main thread listen for socks5 connections
// 2. spawn a child thread for each connection, perform the sock5 and vmess handshake respectively
// 3. after the handshake succeed, spawn a pair of threads to pipe the two connections forward and backward

// todo:
// 1. use thread pool or async io

fn main() {
    let server = Socks5Server::new();
    vmess(&server, Addr::V4([127,0,0,1]), 1080, [219, 131, 173, 224, 50, 114, 78, 197, 160, 203, 164, 175, 6, 31, 23, 48])
}

fn vmess(server: &Socks5Server, proxy_addr: Addr, proxy_port: u16, user_id: [u8; 16]) {
    let connect = Box::leak(Box::new(move |dest, port| {
        let client = std::net::TcpStream::connect(format!("{}:{}", proxy_addr, proxy_port)).unwrap();
        let local = client.local_addr().unwrap();
        let local_addr = match local.ip() {
            std::net::IpAddr::V4(x) => Addr::V4(x.octets()),
            std::net::IpAddr::V6(x) => Addr::V6(x.octets()),
        };
        let local_port = local.port();
        debug!("connect {}:{} through proxy", &dest, port);

        (local_addr, local_port, (dest, port, client))
    }));

    let pass = Box::leak(Box::new(move |(dest, port, conn): (Addr, u16, std::net::TcpStream), stream: std::net::TcpStream| {
        let mut key = [0; 16];
        thread_rng().fill_bytes(&mut key);

        let mut IV = [0; 16];
        thread_rng().fill_bytes(&mut IV);

        {
            let conn = conn.try_clone().unwrap();
            let mut stream = stream.try_clone().unwrap();
            
            std::thread::spawn(move || {
                let mut reader = VmessReader::<std::net::TcpStream>::new(conn, key, IV);
                let mut buffer = [0; 1<<14];
                loop {
                    let len = reader.read(&mut buffer).unwrap();
                    if len == 0 {
                        reader.into_inner().shutdown(std::net::Shutdown::Read).ignore();
                        debug!("closed reading");
                        return 
                    }
                    stream.write_all(&buffer[..len]).unwrap();
                    debug!("read {} bytes", len);
                }
            });
        }

        {
            let conn = conn.try_clone().unwrap();
            let mut stream = stream.try_clone().unwrap();
            std::thread::spawn(move || {
                let mut writer = VmessWriter::new(conn, user_id, dest, port, key, IV);
                let mut buffer = [0; 1<<14];
                loop {
                    let len = stream.read(&mut buffer).unwrap();
                    if len == 0 {
                        writer.write(&[]).unwrap(); // as required by the spec
                        writer.into_inner().shutdown(std::net::Shutdown::Write).ignore();
                        debug!("closed writing");
                        return
                    }
                    writer.write_all(&buffer[..len]).unwrap();
                    debug!("send {} bytes", len);
                }
            });
        }
    }));

    server.listen(connect, pass)
}

fn plain(server: &Socks5Server) {
    server.listen(&|dest, port| {
        let client = std::net::TcpStream::connect(format!("{}:{}", dest, port)).unwrap();
        debug!("connect {}:{}", dest, port);

        let local = client.local_addr().unwrap();
        let local_addr = match local.ip() {
            std::net::IpAddr::V4(x) => Addr::V4(x.octets()),
            std::net::IpAddr::V6(x) => Addr::V6(x.octets()),
        };
        let local_port = local.port();

        (local_addr, local_port, client)
    }, &|proxy, stream| {
        {
            let mut proxy = proxy.try_clone().unwrap();
            let mut stream = stream.try_clone().unwrap();
            std::thread::spawn(move || {
                std::io::copy(&mut proxy, &mut stream)
            });
        }
        {
            let mut proxy = proxy.try_clone().unwrap();
            let mut stream = stream.try_clone().unwrap();
            std::thread::spawn(move || {
                std::io::copy(&mut stream, &mut proxy)
            });
        }
    })
}
