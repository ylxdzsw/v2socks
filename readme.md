V2socks
=======

An opinionated lightweight socks5 server and vmess (v2ray) client implemented in Rust.

#### Deprecation

This software is no longer maintained. (It still works as of Sept. 2024)

You might be interested in another of my project [sopipe](https://github.com/ylxdzsw/sopipe), which also implements the
functionalities in this project, plus much more!

Example of running a socks5 server and a vmess client using `sopipe`:

```sh
$ sopipe 'tcp(1080) => socks5_server => vmess_client("49aa7c07-2cd4-4585-b645-3392fde45b90") => tcp("example.com:3399")'
```

or through HTTP2 over TLS:

```sh
$ sopipe 'tcp(1080) => socks5_server => vmess_client("49aa7c07-2cd4-4585-b645-3392fde45b90") => http2_client("example.com", "/")'
```
