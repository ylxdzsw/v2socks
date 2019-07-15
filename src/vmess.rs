use super::*;
use oh_my_rust::*;
use crypto::digest::Digest;
use crypto::symmetriccipher::BlockEncryptor;
use crypto::mac::Mac;
use std::io::prelude::*;

macro_rules! md5 {
    ($($x:expr),*) => {{
        let mut digest = crypto::md5::Md5::new();
        let mut result = [0; 16];
        $(digest.input($x);)*
        digest.result(&mut result);
        result
    }}
}

/// read from vmess connection and produce decoded stream
#[derive(Debug)]
pub struct VmessReader<R: ReadExt> {
    reader: R,
    decoder: AES128CFB
}

impl VmessReader<std::io::BufReader<std::net::TcpStream>> {
    /// key and IV are just data key and iv in the request header, this function will calculate the md5 it selfs
    #[allow(non_snake_case)]
    pub fn new(conn: std::net::TcpStream, key: [u8; 16], IV: [u8; 16]) -> Option<Self> {
        let mut reader = VmessReader {
            reader: std::io::BufReader::with_capacity(1<<14, conn),
            decoder: AES128CFB::new(md5!(&key), md5!(&IV))
        };
        reader.handshake().ok()?;
        Some(reader)
    }

    pub fn into_inner(self) -> std::net::TcpStream {
        self.reader.into_inner()
    }
}

impl<R: ReadExt> VmessReader<R> {
    fn handshake(&mut self) -> std::io::Result<()> {
        let mut head = [0; 4];

        self.reader.read_exact(&mut head)?;
        self.decoder.decode(&mut head);

        assert!(head[0] == 39); // match the number provided at request handshaking
        let mut cmd = self.reader.read_exact_alloc(head[3] as usize)?;
        self.decoder.decode(&mut cmd);
        Ok(())
    }
}

impl<R: ReadExt> Read for VmessReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let mut temp = [0; 4];
        assert!(buf.len() >= (1<<14) - 4);

        // 1. read and decode length
        if let Err(e) = self.reader.read_exact(&mut temp[..2]) {
            match e.kind() {
                std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset => return Ok(0),
                _ => return Err(e)
            }
        }
        self.decoder.decode(&mut temp[..2]);
        let len = (temp[0] as usize) << 8 | temp[1] as usize;

        // 2. read and decode checksum
        self.reader.read_exact(&mut temp).unwrap();
        self.decoder.decode(&mut temp);

        // 3. read and decode data
        self.reader.read_exact(&mut buf[..len-4]).unwrap();
        self.decoder.decode(&mut buf[..len-4]);

        // 4. verify checksum
        let checksum = fnv1a(&buf[..len-4]);
        if checksum.to_be_bytes() != temp {
            panic!("fuck")
        }

        Ok(len-4)
    }
}

/// write to vmess connection
#[derive(Debug)]
pub struct VmessWriter<W: Write> {
    writer: W,
    encoder: AES128CFB
}

impl VmessWriter<std::net::TcpStream> {
    #[allow(non_snake_case)]
    pub fn new(conn: std::net::TcpStream, user_id: [u8; 16], addr: Addr, port: u16, key: [u8; 16], IV: [u8; 16]) -> Self {
        let mut writer = VmessWriter {
            writer: conn,
            encoder: AES128CFB::new(key, IV)
        };
        writer.handshake(user_id, addr, port, key, IV).unwrap();
        writer
    }

    pub fn into_inner(self) -> std::net::TcpStream {
        self.writer
    }
}

impl<W: Write> VmessWriter<W> {
    #[allow(non_snake_case, non_upper_case_globals)]
    fn handshake(&mut self, user_id: [u8; 16], addr: Addr, port: u16, key: [u8; 16], IV: [u8; 16]) -> std::io::Result<()> {
        let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_be_bytes();
        let mut hmac = crypto::hmac::Hmac::new(crypto::md5::Md5::new(), &user_id);
        hmac.input(&time);
        let mut auth = [0; 16];
        hmac.raw_result(&mut auth);
        self.writer.write_all(&auth)?;

        let mut buffer = Vec::new();

        let version = 1;
        buffer.push(version);

        buffer.extend_from_slice(&IV);
        buffer.extend_from_slice(&key);

        let V = 39; // should be random but who bother
        buffer.push(V);

        let opt = 0b0000_0001;
        buffer.push(opt);

        const P_len: u8 = 0;
        let sec = 0; // AES-128-CFB
        buffer.push((P_len << 4) | (sec & 0x0f));

        let rev = 0; // reserved
        buffer.push(rev);

        let cmd = 1; // tcp
        buffer.push(cmd);

        let port = port.to_be_bytes();
        buffer.extend_from_slice(&port);

        match addr {
            Addr::V4(x) => {
                buffer.push(1);
                buffer.extend_from_slice(&x);
            }
            Addr::V6(x) => {
                buffer.push(3);
                buffer.extend_from_slice(&x);
            },
            Addr::Domain(x) => {
                buffer.push(2);
                buffer.push(x.len() as u8);
                buffer.extend_from_slice(&x);
            }
        }

        let P = [0; P_len as usize];
        buffer.extend_from_slice(&P);

        let F = fnv1a(&buffer);
        buffer.extend_from_slice(&F.to_be_bytes());

        let header_key = md5!(&user_id, b"c48619fe-8f02-49e0-b9e9-edf763e17e21");
        let header_IV = md5!(&time, &time, &time, &time);

        AES128CFB::new(header_key, header_IV).encode(&mut buffer);

        self.writer.write_all(&buffer)?;
        Ok(())
    }
}

impl<W: Write> Write for VmessWriter<W> {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let len = data.len() + 4;
        let mut buf = Vec::with_capacity(len + 2);
        buf.extend_from_slice(&(len as u16).to_be_bytes());
        buf.extend_from_slice(&fnv1a(data).to_be_bytes());
        buf.extend_from_slice(data);
        self.encoder.encode(&mut buf); // this is the right code. the fucking protocol document is misleading!
        self.writer.write_all(&mut buf)?;
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

fn fnv1a(x: &[u8]) -> u32 {
    let prime = 16777619;
    let mut hash = 0x811c9dc5;
    for byte in x.iter() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(prime);
    }
    hash
}

#[derive(Debug)]
struct AES128CFB {
    key: [u8; 16],
    state: [u8; 16],
    p: usize,
}

impl AES128CFB {
    #[allow(non_snake_case)]
    fn new(key: [u8; 16], IV: [u8; 16]) -> AES128CFB {
        AES128CFB { key, state: IV, p: 16 }
    }

    fn encode(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            if self.p == 16 {
                let temp = self.state.clone();
                crypto::aessafe::AesSafe128Encryptor::new(&self.key).encrypt_block(&temp, &mut self.state);
                self.p = 0;
            }
            *byte ^= self.state[self.p];
            self.state[self.p] = *byte;
            self.p += 1;
        }
    }

    fn decode(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            if self.p == 16 {
                let temp = self.state.clone();
                crypto::aessafe::AesSafe128Encryptor::new(&self.key).encrypt_block(&temp, &mut self.state); // yes it's encrypt
                self.p = 0;
            }
            let temp = *byte;
            *byte ^= self.state[self.p];
            self.state[self.p] = temp;
            self.p += 1;
        }
    }
}
