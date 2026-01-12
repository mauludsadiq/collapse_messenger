use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use crate::content::{Message, Content, RetinaBody, StatusEvent};
use crate::types::{PubKey, Digest, now_timestamp, Timestamp};
use crate::reputation::ReputationBook;
use crate::verify::{verify_digest, verify_thread};
use crate::phi::{phi_collapse, assemble_message, Evidence};
use crate::transport::Transport;
use crate::transport_mem::MemoryTransport;

/// Collapse Messenger node with:
/// - inbox of accepted canonical messages
/// - reputation book
/// - retina_store cache
/// - awareness of peers by PubKey
/// - access to a shared transport bus
pub struct NodeMessenger {
    pub id: PubKey,
    pub inbox: Vec<Message>,
    pub rep: ReputationBook,
    pub retina_store: HashMap<Digest, RetinaBody>,

    // who we talk to
    pub peers: Vec<PubKey>,

    // shared transport (for now MemoryTransport via Rc<RefCell<...>>)
    pub bus: Rc<RefCell<MemoryTransport>>,
}

impl NodeMessenger {
    pub fn new(id: PubKey, bus: Rc<RefCell<MemoryTransport>>) -> Self {
        // register ourselves on the bus
        bus.borrow_mut().register_peer(id.clone());

        Self {
            id,
            inbox: Vec::new(),
            rep: ReputationBook::new(),
            retina_store: HashMap::new(),
            peers: Vec::new(),
            bus,
        }
    }

    pub fn add_peer(&mut self, peer_id: PubKey) {
        self.peers.push(peer_id);
    }

    /// User action: produce evidence, collapse (Φ), sign, broadcast.
    /// This is "send a new message into the conversation."
    pub fn send(&mut self, parent: Digest, ev: Evidence) {
        let now = now_timestamp();
        let content = phi_collapse(ev);
        let msg = assemble_message(&self.id, parent, content, now);

        // We always apply our own receive rules locally
        self.receive_internal(&msg);

        // Broadcast to peers (transport-level, not direct calls)
        {
            let mut bus = self.bus.borrow_mut();
            // broadcast to all registered peers other than self
            bus.broadcast(&self.id, &msg);
        }
    }

    /// Poll the transport for inbound messages, run them through
    /// verify_digest / verify_thread / reputation gate / reward/punish.
    pub fn poll(&mut self) {
        // drain messages destined for self.id
        let inbound: Vec<Message> = {
            let mut bus = self.bus.borrow_mut();
            bus.drain_inbound(&self.id)
        };

        for msg in inbound {
            self.receive_internal(&msg);
        }
    }

    /// Send canonical "delivered" or "read" receipts for a given digest.
    pub fn ack_delivered(&mut self, parent_digest: Digest) {
        let now = now_timestamp();
        let evt = StatusEvent::Delivered {
            digest_ack: parent_digest.clone(),
            at: now,
        };
        self.broadcast_status(parent_digest, evt, now);
    }

    pub fn ack_read(&mut self, parent_digest: Digest) {
        let now = now_timestamp();
        let evt = StatusEvent::Read {
            digest_ack: parent_digest.clone(),
            at: now,
        };
        self.broadcast_status(parent_digest, evt, now);
    }

    fn broadcast_status(&mut self, parent_digest: Digest, evt: StatusEvent, now: Timestamp) {
        let ev = Evidence::StatusIntent(evt);
        let content = phi_collapse(ev);
        let msg = assemble_message(&self.id, parent_digest, content, now);

        // apply locally
        self.receive_internal(&msg);

        // send to peers
        {
            let mut bus = self.bus.borrow_mut();
            bus.broadcast(&self.id, &msg);
        }
    }

    /// Core intake:
    /// 1. verify digest/signature
    /// 2. verify causality
    /// 3. verify reputation gate
    /// 4. accept+reward OR reject+punish
    fn receive_internal(&mut self, msg: &Message) -> bool {
        if !verify_digest(msg) {
            self.reject_and_punish(msg, "bad digest/signature");
            return false;
        }

        if !verify_thread(msg, &self.inbox) {
            self.reject_and_punish(msg, "missing parent");
            return false;
        }

        let sender_rep = self.rep.get(&msg.sender);
        if sender_rep < self.rep.admit_threshold() {
            self.reject_and_punish(msg, "sender below trust threshold");
            return false;
        }

        self.accept_and_reward(msg);
        true
    }

    fn accept_and_reward(&mut self, msg: &Message) {
        // store message
        self.inbox.push(msg.clone());

        // cache retinal witness for resurrection
        if let Content::Retina(ref r) = msg.content {
            self.retina_store.insert(msg.digest.clone(), r.clone());
        }

        // reward sender
        self.rep.reward(&msg.sender);
    }

    fn reject_and_punish(&mut self, msg: &Message, reason: &str) {
        eprintln!(
            "⚠️ {} rejects {:?}: {}",
            self.id.0,
            msg.digest,
            reason
        );
        self.rep.punish(&msg.sender);
    }

    /// Deterministic healing:
    /// Instead of asking Rc peers directly, we now just
    /// re-run causal acceptance as new messages arrive via poll().
    /// With a real network, "heal" becomes:
    /// - request thread history,
    /// - replay them through receive_internal in order.
    ///
    /// We'll keep a stub here in case we pipeline it later.
    pub fn heal(&mut self) {
        // No-op for now: poll() + replay is our heal mechanism in transport mode.
        // Future: ask bus (or peer) for missing history by digest.
    }

    pub fn decay_reputation(&mut self) {
        self.rep.decay();
    }
}
