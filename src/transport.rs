use crate::types::PubKey;
use crate::content::Message;

// Transport is how nodes send messages to peers.
// Each NodeMessenger will hold a Box<dyn Transport>.
pub trait Transport {
    // send one canonical message to a specific peer identity
    fn send_to(&mut self, to: &PubKey, msg: &Message);

    // broadcast one canonical message to all known peers
    fn broadcast(&mut self, from: &PubKey, msg: &Message);

    // (pull) get all inbound messages destined for `me`
    fn drain_inbound(&mut self, me: &PubKey) -> Vec<Message>;
}
