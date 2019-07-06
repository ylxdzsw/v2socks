use v2socks::*;
use std::io::prelude::*;

// basic logic:
// 1. main thread listen for socks5 connections
// 2. spawn a child thread for each connection, perform the sock5 and vmess handshake respectively
// 3. after the handshake succeed, spawn a pair of threads to pipe the two connections forward and backward

fn main() {
    // let server = Socks5Server::new();

    // server.listen(&|dest, port| {
    //     let client = std::net::TcpStream::connect(format!("{}:{}", dest, port)).unwrap();
    //     let local = client.local_addr().unwrap();
    //     let local_addr = match local.ip() {
    //         std::net::IpAddr::V4(x) => Addr::V4(x.octets()),
    //         std::net::IpAddr::V6(x) => Addr::V6(x.octets()),
    //     };
    //     let local_port = local.port();

    //     (local_addr, local_port, client)
    // }, &|proxy, stream| {
        
    // })

    let (auth, cmd) = v2socks::vmess::request(
        [219, 131, 173, 224, 50, 114, 78, 197, 160, 203, 164, 175, 6, 31, 23, 48],
        Addr::Domain("www.google.com".as_bytes().to_owned().into_boxed_slice()),
        443
    );

    let mut sock = std::net::TcpStream::connect("setsuna.v2.cccat.io:8080").unwrap();
    println!("about to write");
    sock.write(&auth).unwrap();
    // sock.write(&cmd).unwrap();

    loop {
        let mut x = [0; 1024];
        println!("about to read");
        let n = sock.read(&mut x).unwrap();
        println!("after read");
        if n == 0 {
            break
        }
        println!("{:?}", Vec::new().extend_from_slice(&x));
    }

    println!("finished")
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