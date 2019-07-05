use super::*;
use oh_my_rust::*;
use crypto::digest::Digest;
use crypto::mac::Mac;
use crypto::sha3::Sha3;
use rand::prelude::*;

pub struct VmessClient {

}

impl VmessClient {
    pub fn new() -> VmessClient {
        unimplemented!()
    }

}

#[allow(non_snake_case, non_upper_case_globals)]
pub fn request(user_id: [u8; 16], addr: Addr, port: u16) -> Box<[u8]> {
    let mut buffer = Vec::new();

    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_be_bytes();
    let mut hmac = crypto::hmac::Hmac::new(crypto::md5::Md5::new(), &user_id);
    hmac.input(&time);
    let mut auth = [0; 16];
    hmac.raw_result(&mut auth);
    debug!("{:?}", auth);
    buffer.extend_from_slice(&auth);

    let version = 1;
    buffer.push(version);

    let mut md5 = crypto::md5::Md5::new();
    let mut IV = [0; 16];
    for _ in 0..4 {
        md5.input(&time)
    }
    md5.result(&mut IV);
    buffer.extend_from_slice(&IV);

    let mut enc_key = [0; 16];
    thread_rng().fill_bytes(&mut enc_key);
    buffer.extend_from_slice(&enc_key);

    let V = 39; // should be random but who bother
    buffer.push(V);

    let opt = 0b0000_0001;
    buffer.push(opt);

    const P_len: u8 = 1;
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

    buffer.into_boxed_slice()
}

fn shake() {
    let mut hasher = Sha3::shake_128();
    hasher.input(b" fuck them");
    let hex = hasher.result_str();
    unimplemented!()
}

fn md5(x: &[u8]) -> [u8; 16] {
    let mut digest = crypto::md5::Md5::new();
    let mut result = [0; 16];
    digest.input(x);
    digest.result(&mut result);
    result
}

fn fnv1a(x: &[u8]) -> u32 {
    let prime = 16777619;
    let mut hash = 0x811c9dc5;
    for byte in x.iter() {
        hash ^= *byte as u32;
        hash *= prime;
    }
    hash
}