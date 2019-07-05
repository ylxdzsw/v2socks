use v2socks::*;

// basic logic:
// 1. main thread listen for socks5 connections
// 2. spawn a child thread for each connection, perform the sock5 and vmess handshake respectively
// 3. after the handshake succeed, add both stream to the global pipe poll

// roadmap:
// 1. remove token table in piper and use direct pointers instead.
// 2. split read and write and fully asynconize IO

fn main() {
    let server = Socks5Server::new();
    server.listen(&|dest, port| {
        let client = std::net::TcpStream::connect(format!("{}:{}", dest, port)).unwrap();
        let local = client.local_addr().unwrap();
        let local_addr = match local.ip() {
            std::net::IpAddr::V4(x) => Addr::V4(x.octets()),
            std::net::IpAddr::V6(x) => Addr::V6(x.octets()),
        };
        let local_port = local.port();
        
        (local_addr, local_port, client)
    }, &|proxy, stream| {
        let proxy = std::sync::Arc::new(proxy);
        let stream = std::sync::Arc::new(proxy);

        std::thread::spawn(move || {
            let p = &proxy.clone();

        });

        std::thread::spawn(move || {

        });
    })
}
