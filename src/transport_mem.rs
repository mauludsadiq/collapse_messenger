use std::collections::{HashMap, HashSet};
use crate::types::PubKey;
use crate::content::Message;
use crate::transport::Transport;

/// MemoryTransport is a shared in-memory message bus.
/// Each registered PubKey gets a queue. send_to() enqueues to one.
/// broadcast() enqueues to all peers except the sender.
/// drain_inbound() hands a node its queued messages.
pub struct MemoryTransport {
    peers: HashSet<PubKey>,
    queues: HashMap<PubKey, Vec<Message>>,
}

impl MemoryTransport {
    pub fn new() -> Self {
        Self {
            peers: HashSet::new(),
            queues: HashMap::new(),
        }
    }

    pub fn register_peer(&mut self, who: PubKey) {
        self.peers.insert(who.clone());
        self.queues.entry(who).or_insert_with(Vec::new);
    }

    fn enqueue(&mut self, to: &PubKey, msg: &Message) {
        if let Some(q) = self.queues.get_mut(to) {
            q.push(msg.clone());
        }
    }
}

impl Transport for MemoryTransport {
    fn send_to(&mut self, to: &PubKey, msg: &Message) {
        if self.peers.contains(to) {
            self.enqueue(to, msg);
        }
    }

    fn broadcast(&mut self, from: &PubKey, msg: &Message) {
        // Step 1: snapshot peers so we don't alias-borrow self.peers
        // while mutating self.queues.
        let targets: Vec<PubKey> = self
            .peers
            .iter()
            .filter(|p| *p != from)
            .cloned()
            .collect();

        // Step 2: now it's safe to mutate self.queues
        for peer_id in targets {
            self.enqueue(&peer_id, msg);
        }
    }

    fn drain_inbound(&mut self, me: &PubKey) -> Vec<Message> {
        if let Some(q) = self.queues.get_mut(me) {
            let drained = q.clone();
            q.clear();
            drained
        } else {
            Vec::new()
        }
    }
}
