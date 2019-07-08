use super::*;
use oh_my_rust::*;
use crypto::digest::Digest;
use crypto::symmetriccipher::BlockEncryptor;
use crypto::mac::Mac;
use rand::prelude::*;

macro_rules! md5 {
    ($($x:expr),*) => {{
        let mut digest = crypto::md5::Md5::new();
        let mut result = [0; 16];
        $(digest.input($x);)*
        digest.result(&mut result);
        result
    }}
}

pub struct VmessClient {

}

impl VmessClient {
    pub fn new() -> VmessClient {
        unimplemented!()
    }

}

#[allow(non_snake_case, non_upper_case_globals)]
fn request(user_id: [u8; 16], addr: Addr, port: u16) -> ([u8; 16], Box<[u8]>) {
    let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().to_be_bytes();
    let mut hmac = crypto::hmac::Hmac::new(crypto::md5::Md5::new(), &user_id);
    hmac.input(&time);
    let mut auth = [0; 16];
    hmac.raw_result(&mut auth);
    debug!("{:?}", auth);

    let mut buffer = Vec::new();

    let version = 1;
    buffer.push(version);

    let mut enc_IV_and_key = [0; 32];
    thread_rng().fill_bytes(&mut enc_IV_and_key);
    buffer.extend_from_slice(&enc_IV_and_key);

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

    // let P = [0; P_len as usize];
    // buffer.extend_from_slice(&P);

    let F = fnv1a(&buffer);
    buffer.extend_from_slice(&F.to_be_bytes());

    debug!("{:?}", buffer);

    let key = md5!(&user_id, b"c48619fe-8f02-49e0-b9e9-edf763e17e21");
    let IV = md5!(&time, &time, &time, &time);

    debug!("key {:?}", key);
    debug!("IV {:?}", IV);

    aes128cfb_encode(&mut buffer, key, IV);
    debug!("{:?}", buffer);

    (auth, buffer.into_boxed_slice())
}

fn fnv1a(x: &[u8]) -> u32 { // checked
    let prime = 16777619;
    let mut hash = 0x811c9dc5;
    for byte in x.iter() {
        hash ^= *byte as u32;
        hash = hash.wrapping_mul(prime);
    }
    hash
}

#[allow(non_snake_case)]
fn aes128cfb_encode(data: &mut [u8], key: [u8; 16], IV: [u8; 16]) { // checked, match the output of cfb_mode crate.
    let mut hash = IV;
    let mut temp = [0; 16];

    for chunk in data.chunks_mut(16) {
        crypto::aessafe::AesSafe128Encryptor::new(&key).encrypt_block(&hash, &mut temp);
        for ((x, y), z) in chunk.iter_mut().zip(&mut temp).zip(&mut hash) {
            *x ^= *y;
            *z = *x;
        }
    }
}

#[allow(non_snake_case)]
fn aes128cfb_decode(data: &mut [u8], key: [u8; 16], IV: [u8; 16]) {
    let mut hash = IV;
    let mut temp = [0; 16];
    
    for chunk in data.chunks_mut(16) {
        crypto::aessafe::AesSafe128Encryptor::new(&key).encrypt_block(&hash, &mut temp); // Yes it's *encrypt* here
        for ((x, y), z) in chunk.iter_mut().zip(&mut temp).zip(&mut hash) {
            *z = *x; // the order here is the only difference with encoding
            *x ^= *y;
        }
    }
}