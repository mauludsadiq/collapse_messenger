use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Digest as ShaDigest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Digest(pub [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PubKey(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Signature(pub String);

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Timestamp(pub u128);

pub fn now_timestamp() -> Timestamp {
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    Timestamp(dur.as_millis())
}

pub fn compute_digest<T: ?Sized + Serialize>(obj: &T) -> Digest {
    let json = serde_json::to_string(obj).unwrap();
    let mut h = Sha256::new();
    h.update(json.as_bytes());
    let out = h.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&out[..32]);
    Digest(bytes)
}

pub fn sign_digest(pk: &PubKey, d: &Digest) -> Signature {
    Signature(format!("SIG{{{}::{:?}}}", pk.0, d.0))
}

pub fn zero_digest() -> Digest {
    Digest([0u8; 32])
}
