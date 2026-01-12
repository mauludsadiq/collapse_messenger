use serde::{Serialize, Deserialize};
use crate::content::Message;

#[derive(Debug, Serialize, Deserialize)]
pub struct WireMessage {
    pub message: Message,
}

// Serialize to JSON string for transport.
pub fn encode_message(msg: &Message) -> String {
    let wm = WireMessage { message: msg.clone() };
    serde_json::to_string(&wm).expect("wire encode")
}

// Deserialize from JSON string received over transport.
pub fn decode_message(s: &str) -> Option<Message> {
    let parsed: WireMessage = serde_json::from_str(s).ok()?;
    Some(parsed.message)
}
