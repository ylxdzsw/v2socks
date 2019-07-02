use crypto::digest::Digest;
use crypto::sha3::Sha3;

fn shake() {
    let mut hasher = Sha3::shake_128();
    hasher.input(b" fuck them");
    let hex = hasher.result_str();
    unimplemented!()
}
