use crate::content::Message;
use crate::types::{compute_digest, Digest, Signature, zero_digest};

pub fn verify_digest(msg: &Message) -> bool {
    let d_local = compute_digest(&msg.content);
    if d_local != msg.digest {
        return false;
    }
    let expected_sig = Signature(format!("SIG{{{}::{:?}}}", msg.sender.0, msg.digest.0));
    msg.signature == expected_sig
}

pub fn verify_thread(msg: &Message, inbox: &[Message]) -> bool {
    if msg.parent == zero_digest() {
        return true;
    }
    inbox.iter().any(|m| m.digest == msg.parent)
}
