use v2socks::{Socks5Server, VmessClient};

fn main() {
    let server = Socks5Server::new();
    server.listen(|dest, port, stream| {
        let client = VmessClient::new(dest, port);
        client.pass(stream);
    })
}
