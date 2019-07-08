use v2socks::*;

// basic logic:
// 1. main thread listen for socks5 connections
// 2. spawn a child thread for each connection, perform the sock5 and vmess handshake respectively
// 3. after the handshake succeed, spawn a pair of threads to pipe the two connections forward and backward

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
        // sock.shutdown(std::net::Shutdown::Write).unwrap();
    })
}

// let plain = |proxy, stream| {
//     {
//         let mut proxy = proxy.try_clone().unwrap();
//         let mut stream = stream.try_clone().unwrap();
//         std::thread::spawn(move || {
//             std::io::copy(&mut proxy, &mut stream)
//         });
//     }
//     {
//         let mut proxy = proxy.try_clone().unwrap();
//         let mut stream = stream.try_clone().unwrap();
//         std::thread::spawn(move || {
//             std::io::copy(&mut stream, &mut proxy)
//         });
//     }
// }