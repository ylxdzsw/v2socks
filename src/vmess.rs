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

#[allow(non_snake_case)]
fn request(user_id: [u8; 16], addr: Addr, port: u16) {
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

    let P = [0; 4]; // yet another uneccesary random value
    buffer.extend_from_slice(&P);

    let sec = 0; // AES-128-CFB
    buffer.push(sec);

    let rev = 0; // reserved
    buffer.push(rev);

    let cmd = 1; // tcp
    buffer.push(cmd);


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